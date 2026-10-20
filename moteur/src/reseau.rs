//! Jeu en ligne de 2 à 5 joueurs, connexion TCP directe par IP, en étoile (l'hôte relaie).
//!
//! Principe « lockstep » : toutes les machines exécutent la même partie (mêmes règles, mêmes bots, même
//! générateur aléatoire dont la graine est envoyée par l'hôte au démarrage). Seules les actions des humains
//! circulent, 12 octets chacune. L'hôte est le siège 0 ; chaque jeu choisit les sièges de ses invités
//! (ex. belote à 4 : le 1er invité est le partenaire). Les sièges libres sont tenus par des bots.
//!
//! Trames : 1 = bonjour (siège attribué à l'invité), 2 = départ (configuration + graine), 3 et plus = actions
//! du jeu.

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

pub const PORT: u16 = 4210;
pub const T_BONJOUR: u8 = 1;
pub const T_DEPART: u8 = 2;
/// Premier numéro libre pour les actions d'un jeu.
pub const T_ACTION: u8 = 3;

const PERDUE: &str = "Connexion perdue : retour à la partie locale.";

/// Trame de 12 octets : type, octet, mot de 16 bits, deux mots de 32 bits.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Frame {
    pub kind: u8,
    pub b1: u8,
    pub b: u16,
    pub c: u32,
    pub d: u32,
}

pub fn encode(f: &Frame) -> [u8; 12] {
    let mut b = [0u8; 12];
    b[0] = f.kind;
    b[1] = f.b1;
    b[2..4].copy_from_slice(&f.b.to_le_bytes());
    b[4..8].copy_from_slice(&f.c.to_le_bytes());
    b[8..12].copy_from_slice(&f.d.to_le_bytes());
    b
}

pub fn decode(b: &[u8]) -> Frame {
    Frame {
        kind: b[0],
        b1: b[1],
        b: u16::from_le_bytes([b[2], b[3]]),
        c: u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        d: u32::from_le_bytes([b[8], b[9], b[10], b[11]]),
    }
}

/// « ip » ou « ip:port » -> adresse (port 4210 par défaut).
pub fn parse_addr(s: &str) -> Option<SocketAddr> {
    let (ip, port) = match s.split_once(':') {
        Some((ip, p)) => (ip, p.parse::<u16>().ok().filter(|&p| p != 0)?),
        None => (s, PORT),
    };
    let ip: std::net::Ipv4Addr = ip.parse().ok()?;
    if ip.is_unspecified() {
        return None;
    }
    Some(SocketAddr::from((ip, port)))
}

/// Adresse IP locale principale (celle que les amis du réseau local doivent taper).
fn local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("192.0.2.1:9")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "?".into())
}

/// Ce qu'un jeu fournit pour être joué en ligne.
pub trait Partie {
    /// Sièges donnés aux invités dans l'ordre d'arrivée ; sa longueur = nombre maximal d'invités.
    fn sieges_invites(&self) -> Vec<usize>;
    /// Configuration envoyée aux invités au départ (un octet, un mot), selon les sièges humains (masque).
    fn configuration(&self, humains: u32) -> (u8, u16);
    /// Démarre la partie en ligne au siège donné (identique sur toutes les machines).
    fn demarrer(&mut self, siege: usize, b1: u8, b: u16, graine: u64);
    /// Fin du jeu en ligne : retour à une partie locale.
    fn quitter(&mut self);
    fn en_ligne(&self) -> bool;
    /// Actions locales à envoyer aux autres.
    fn sortantes(&mut self) -> Vec<Frame>;
    /// Action reçue d'un autre joueur.
    fn recue(&mut self, f: Frame);
    /// Un nombre aléatoire local (mélangé à l'horloge pour la graine de la partie).
    fn graine_locale(&self) -> u64;
    /// Identifiant du jeu et de sa version : un invité refuse un hôte qui joue à autre chose.
    fn jeu(&self) -> u32;
}

struct Peer {
    s: TcpStream,
    rx: Vec<u8>,
}

impl Peer {
    fn new(s: TcpStream) -> Peer {
        let _ = s.set_nodelay(true);
        let _ = s.set_nonblocking(true);
        Peer { s, rx: Vec::new() }
    }

    fn send(&mut self, f: &Frame) -> bool {
        let buf = encode(f);
        let mut done = 0;
        for _ in 0..1000 {
            match self.s.write(&buf[done..]) {
                Ok(n) => {
                    done += n;
                    if done == buf.len() {
                        return true;
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                    std::thread::sleep(Duration::from_millis(1))
                }
                Err(_) => return false,
            }
        }
        false
    }

    /// Lit ce qui est arrivé ; renvoie les trames complètes, ou None si la connexion est fermée.
    fn recv(&mut self) -> Option<Vec<Frame>> {
        let mut buf = [0u8; 256];
        loop {
            match self.s.read(&mut buf) {
                Ok(0) => return None,
                Ok(n) => self.rx.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => return None,
            }
        }
        let mut out = Vec::new();
        while self.rx.len() >= 12 {
            out.push(decode(&self.rx[..12]));
            self.rx.drain(..12);
        }
        Some(out)
    }
}

enum State {
    Off,
    /// hôte : attente des invités
    Hosting { l: TcpListener, guests: Vec<Peer> },
    Connecting(Receiver<std::io::Result<TcpStream>>),
    /// invité connecté : attente du départ (siège reçu dans BONJOUR)
    Waiting { host: Peer, seat: usize },
    /// partie en ligne : l'hôte a les invités, l'invité a l'hôte
    Game { peers: Vec<Peer>, host: bool },
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Menu {
    None,
    Choice,
    Ip,
    /// message d'information : une touche le ferme
    Info,
}

pub struct Net {
    state: State,
    pub menu: Menu,
    /// dernière adresse saisie (à mémoriser dans les réglages)
    pub ip: String,
    pub lines: [String; 3],
}

impl Net {
    pub fn new(ip: String) -> Net {
        Net { state: State::Off, menu: Menu::None, ip, lines: Default::default() }
    }

    /// Attente en ligne (hôte, connexion, invité en attente) : le panneau est affiché, Échap annule.
    pub fn waiting(&self) -> bool {
        matches!(self.state, State::Hosting { .. } | State::Connecting(_) | State::Waiting { .. })
    }

    pub fn hosting(&self) -> bool {
        matches!(self.state, State::Hosting { .. })
    }

    /// Nombre d'invités connectés (hôte en attente).
    pub fn invites(&self) -> usize {
        match &self.state {
            State::Hosting { guests, .. } => guests.len(),
            State::Game { peers, host: true } => peers.len(),
            _ => 0,
        }
    }

    /// Partie en ligne en cours.
    pub fn en_partie(&self) -> bool {
        matches!(self.state, State::Game { .. })
    }

    /// Panneau à afficher : (titre, lignes, mode menu « H / R »).
    pub fn panel(&self) -> Option<(&str, [&str; 3], bool)> {
        if self.menu == Menu::Choice {
            return Some((
                "MULTIJOUEUR EN LIGNE",
                [
                    "H : héberger une partie      R : rejoindre une partie      Echap : annuler",
                    "Tous les joueurs doivent avoir la même version du jeu.",
                    "",
                ],
                true,
            ));
        }
        if self.menu != Menu::None || self.waiting() {
            return Some(("MULTIJOUEUR EN LIGNE", [&self.lines[0], &self.lines[1], &self.lines[2]], false));
        }
        None
    }

    fn say(&mut self, a: &str, b: &str, c: &str) {
        self.lines = [a.into(), b.into(), c.into()];
    }

    /// Touche N : ouvre le menu, ou coupe la connexion en cours.
    pub fn open(&mut self, p: &mut impl Partie) {
        if !matches!(self.state, State::Off) {
            self.close(p, PERDUE);
            return;
        }
        if self.menu == Menu::None {
            self.menu = Menu::Choice;
            self.say("", "", "");
        }
    }

    /// Touche pendant le menu « H / R » (toute autre touche annule).
    pub fn menu_key(&mut self, p: &mut impl Partie, c: char) {
        match c.to_ascii_uppercase() {
            'H' => {
                self.menu = Menu::None;
                self.host(p);
            }
            'R' => {
                self.menu = Menu::Ip;
                self.prompt();
            }
            _ => self.menu = Menu::None,
        }
    }

    fn prompt(&mut self) {
        let l1 = format!("Adresse IP de l'hôte (Entrée = valider) : {}_", self.ip);
        self.say(&l1, "Exemple : 192.168.1.20   (autre port : 192.168.1.20:4210). Echap : annuler.", "");
    }

    /// Saisie de l'adresse (Échap = '\u{1b}', Entrée = '\r', retour arrière = '\u{8}').
    /// Renvoie vrai quand l'adresse est validée (à mémoriser).
    pub fn ip_char(&mut self, c: char) -> bool {
        match c {
            '\u{1b}' => self.menu = Menu::None,
            '\r' | '\n' => return self.join(),
            '\u{8}' => {
                self.ip.pop();
                self.prompt();
            }
            '0'..='9' | '.' | ':' if self.ip.len() < 21 => {
                self.ip.push(c);
                self.prompt();
            }
            _ => {}
        }
        false
    }

    fn host_msg(&mut self, p: &impl Partie, n: usize) {
        self.lines[2] = if p.sieges_invites().len() <= 1 {
            "Partie à deux : elle démarre dès qu'un joueur se connecte.".into()
        } else {
            format!("{} joueur(s) connecté(s). Entrée : commencer maintenant (les places libres sont tenues par des bots).", n + 1)
        };
    }

    fn host(&mut self, p: &impl Partie) {
        let l = TcpListener::bind(("0.0.0.0", PORT)).and_then(|l| {
            l.set_nonblocking(true)?;
            Ok(l)
        });
        match l {
            Ok(l) => {
                self.state = State::Hosting { l, guests: Vec::new() };
                let l1 = format!("En attente de joueurs...  Ton adresse IP : {}", local_ip());
                self.say(&l1, "Port 4210 (réseau local, VPN type Tailscale, ou redirection de port). Echap : annuler.", "");
                self.host_msg(p, 0);
            }
            Err(_) => {
                self.say("Impossible d'écouter sur le port 4210 (déjà utilisé ?).", "", "");
                self.menu = Menu::Info;
            }
        }
    }

    fn join(&mut self) -> bool {
        let Some(addr) = parse_addr(&self.ip) else {
            self.prompt();
            self.lines[1] = "Adresse IP invalide. Exemple : 192.168.1.20".into();
            return false;
        };
        self.menu = Menu::None;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(TcpStream::connect_timeout(&addr, Duration::from_secs(8)));
        });
        self.state = State::Connecting(rx);
        self.say("Connexion en cours... (Echap : annuler)", "", "");
        true
    }

    /// Hôte : lance la partie avec les invités présents (Entrée, ou table complète).
    pub fn host_start(&mut self, p: &mut impl Partie) {
        let State::Hosting { guests, .. } = &mut self.state else { return };
        if guests.is_empty() {
            return;
        }
        let seats = p.sieges_invites();
        let mut humans = 1u32;
        for k in 0..guests.len() {
            humans |= 1 << seats[k];
        }
        let (b1, b) = p.configuration(humans);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let seed = (p.graine_locale() ^ now as u64) | 1;
        let start = Frame { kind: T_DEPART, b1, b, c: seed as u32, d: (seed >> 32) as u32 };
        let mut peers = std::mem::take(guests);
        let ok = peers.iter_mut().all(|g| g.send(&start));
        self.state = State::Game { peers, host: true };
        p.demarrer(0, b1, b, seed);
        if !ok {
            self.close(p, PERDUE);
        }
    }

    /// Ferme tout ; une partie en ligne laisse place à une partie locale et à un message.
    pub fn close(&mut self, p: &mut impl Partie, msg: &str) {
        let was_game = matches!(self.state, State::Game { .. });
        self.state = State::Off;
        self.menu = Menu::None;
        if was_game {
            p.quitter();
            self.say(msg, "", "");
            self.menu = Menu::Info;
        }
    }

    /// Échap pendant une attente.
    pub fn cancel(&mut self, p: &mut impl Partie) {
        self.close(p, "");
        self.menu = Menu::None;
    }

    /// À appeler régulièrement (à chaque tic de la boucle).
    pub fn poll(&mut self, p: &mut impl Partie) {
        match &mut self.state {
            State::Off => {
                let _ = p.sortantes();
            }
            State::Hosting { l, guests } => {
                let seats = p.sieges_invites();
                if let Ok((s, _)) = l.accept() {
                    if guests.len() < seats.len() {
                        let seat = seats[guests.len()];
                        let mut peer = Peer::new(s);
                        if peer.send(&Frame { kind: T_BONJOUR, b1: seat as u8, b: 0, c: p.jeu(), d: 0 }) {
                            guests.push(peer);
                        }
                    }
                    let n = guests.len();
                    self.host_msg(p, n);
                    if n >= seats.len() {
                        self.host_start(p);
                        return;
                    }
                }
                // un invité parti avant le départ : on le retire
                if let State::Hosting { guests, .. } = &mut self.state {
                    let before = guests.len();
                    guests.retain_mut(|g| g.recv().is_some());
                    if guests.len() != before {
                        let n = guests.len();
                        self.host_msg(p, n);
                    }
                }
            }
            State::Connecting(rx) => match rx.try_recv() {
                Ok(Ok(s)) => {
                    self.state = State::Waiting { host: Peer::new(s), seat: 1 };
                    self.say("Connecté ! En attente du départ de la partie par l'hôte...", "", "");
                }
                Ok(Err(_)) | Err(TryRecvError::Disconnected) => {
                    self.state = State::Off;
                    self.say("Connexion impossible (hôte absent ou port fermé).", "", "");
                    self.menu = Menu::Info;
                }
                Err(TryRecvError::Empty) => {}
            },
            State::Waiting { host, seat } => match host.recv() {
                None => {
                    self.state = State::Off;
                    self.say(PERDUE, "", "");
                    self.menu = Menu::Info;
                }
                Some(frames) => {
                    for (k, f) in frames.iter().copied().enumerate() {
                        if f.kind == T_BONJOUR {
                            if f.c != p.jeu() {
                                self.state = State::Off;
                                self.say("L'hôte joue à un autre jeu ou à une autre version.", "", "");
                                self.menu = Menu::Info;
                                return;
                            }
                            *seat = f.b1 as usize;
                        } else if f.kind == T_DEPART {
                            let seed = f.c as u64 | (f.d as u64) << 32;
                            let me = *seat;
                            let State::Waiting { host, .. } = std::mem::replace(&mut self.state, State::Off) else {
                                unreachable!()
                            };
                            self.state = State::Game { peers: vec![host], host: false };
                            p.demarrer(me, f.b1, f.b, seed);
                            // actions arrivées dans le même paquet que le départ
                            for a in frames[k + 1..].iter().filter(|a| a.kind >= T_ACTION) {
                                p.recue(*a);
                            }
                            return;
                        }
                    }
                }
            },
            State::Game { peers, host } => {
                let mut lost = false;
                // réception (l'hôte relaie aux autres invités)
                for i in 0..peers.len() {
                    let Some(frames) = peers[i].recv() else {
                        lost = true;
                        break;
                    };
                    for f in frames {
                        if f.kind >= T_ACTION {
                            p.recue(f);
                            if *host {
                                for (j, other) in peers.iter_mut().enumerate() {
                                    if j != i && !other.send(&f) {
                                        lost = true;
                                    }
                                }
                            }
                        }
                    }
                }
                // envoi des actions locales à tous
                for f in &p.sortantes() {
                    for other in peers.iter_mut() {
                        if !other.send(f) {
                            lost = true;
                        }
                    }
                }
                if lost {
                    self.close(p, PERDUE);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trames() {
        let f = Frame { kind: T_DEPART, b1: 0x9F, b: 1000, c: 0xDEADBEEF, d: 0x01234567 };
        assert_eq!(decode(&encode(&f)), f);
        assert_eq!(parse_addr("192.168.1.20"), Some("192.168.1.20:4210".parse().unwrap()));
        assert_eq!(parse_addr("10.0.0.1:5000"), Some("10.0.0.1:5000".parse().unwrap()));
        assert_eq!(parse_addr("10.0.0"), None);
        assert_eq!(parse_addr("0.0.0.0"), None);
    }
}
