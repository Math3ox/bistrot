//! Fenêtre et boucle du jeu, sans carte graphique : winit pour la fenêtre et les entrées ; l'image calculée
//! par le processeur est affichée directement par GDI sous Windows (aucun tampon en double), par softbuffer
//! ailleurs (X11 / Wayland).
//!
//! Mémoire : **un jeu ne dépasse jamais 50 Mo**. Deux images seulement (le fond en cache et l'image en
//! cours), à une résolution interne plafonnée (au-delà de ~2,2 millions de pixels, l'image est calculée plus
//! petite puis agrandie par GDI) ; un garde-fou mesure la mémoire du processus et allège les caches du jeu,
//! puis la résolution interne, si elle approche de la limite.
//!
//! Le fond (décor + tout ce qui ne bouge pas) est refait seulement quand son empreinte ou la taille change ;
//! chaque image recopie ce fond puis dessine ce qui bouge. Rien n'est redessiné quand rien ne change.

use crate::gfx::{Gfx, Glyphs, WH, WW};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tiny_skia::Pixmap;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WKey, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

/// Limite de mémoire d'un jeu (octets) et seuil d'alerte du garde-fou.
pub const LIMITE_RAM: usize = 50 << 20;
const ALERTE_RAM: usize = 44 << 20;
/// Pixels de l'image interne au plus (1920 x 1080 = 2,07 millions).
const PIXELS_MAX: f64 = 2.2e6;

/// Touches utiles au jeu.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Key {
    Char(char),
    Esc,
    F11,
    Space,
    Enter,
    Back,
}

pub trait Game {
    fn title(&self) -> String;
    /// Nom du fichier de capture (F12).
    fn shot_name(&self) -> &str;
    /// Empreinte du fond : quand elle change, `background` est redessiné (sinon le fond reste en cache).
    fn fond_signature(&self) -> u64 {
        0
    }
    /// Fond : décor et tout ce qui ne bouge pas (mis en cache).
    fn background(&mut self, g: &Gfx);
    /// Tout ce qui se dessine par-dessus le fond.
    fn draw(&mut self, g: &Gfx);
    /// Avance le temps ; renvoie vrai si l'image a changé.
    fn tick(&mut self, dt_ms: f64) -> bool;
    /// Une animation est en cours (images rapprochées).
    fn animating(&self) -> bool;
    fn key(&mut self, k: Key);
    /// Clic gauche (coordonnées logiques 1100 x 800).
    fn click(&mut self, x: f32, y: f32);
    /// Souris déplacée ; renvoie vrai si l'image change (survol).
    fn hover(&mut self, x: f32, y: f32) -> bool;
    /// Plein écran voulu par le jeu.
    fn fullscreen(&self) -> bool;
    fn quit(&self) -> bool;
    /// La mémoire approche de la limite : vider les caches (ils se refont à la demande).
    fn alleger(&mut self) {}
}

/// Mémoire du processus (octets de l'ensemble de travail), si le système sait la donner.
pub fn memoire() -> Option<usize> {
    #[cfg(windows)]
    {
        gdi::memoire()
    }
    #[cfg(not(windows))]
    {
        let s = std::fs::read_to_string("/proc/self/statm").ok()?;
        let pages: usize = s.split_whitespace().nth(1)?.parse().ok()?;
        Some(pages * 4096)
    }
}

#[cfg(windows)]
mod gdi {
    use std::ffi::c_void;
    use tiny_skia::Pixmap;

    #[repr(C)]
    struct BitmapInfo {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        xppm: i32,
        yppm: i32,
        clr_used: u32,
        clr_important: u32,
        couleurs: [u32; 4],
    }

    #[repr(C)]
    #[derive(Default)]
    struct Compteurs {
        cb: u32,
        defauts: u32,
        pic: usize,
        travail: usize,
        _autres: [usize; 6],
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetDC(hwnd: isize) -> isize;
        fn ReleaseDC(hwnd: isize, hdc: isize) -> i32;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn StretchDIBits(
            hdc: isize,
            xd: i32,
            yd: i32,
            wd: i32,
            hd: i32,
            xs: i32,
            ys: i32,
            ws: i32,
            hs: i32,
            bits: *const c_void,
            bmi: *const BitmapInfo,
            usage: u32,
            rop: u32,
        ) -> i32;
        fn SetStretchBltMode(hdc: isize, mode: i32) -> i32;
        fn SetBrushOrgEx(hdc: isize, x: i32, y: i32, pt: *mut c_void) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(p: isize, c: *mut Compteurs, cb: u32) -> i32;
    }

    /// Affiche l'image (agrandie à w x h si besoin). L'image est convertie sur place en BGRA :
    /// elle n'est plus utilisable ensuite (elle est refaite à partir du fond à l'image suivante).
    pub fn afficher(hwnd: isize, pm: &mut Pixmap, w: i32, h: i32) {
        let (pw, ph) = (pm.width() as i32, pm.height() as i32);
        for px in pm.data_mut().chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        let bmi = BitmapInfo {
            size: 40,
            width: pw,
            height: -ph, // de haut en bas
            planes: 1,
            bit_count: 32,
            compression: 0,
            size_image: 0,
            xppm: 0,
            yppm: 0,
            clr_used: 0,
            clr_important: 0,
            couleurs: [0; 4],
        };
        unsafe {
            let hdc = GetDC(hwnd);
            if hdc == 0 {
                return;
            }
            if pw != w || ph != h {
                SetStretchBltMode(hdc, 4); // HALFTONE : agrandissement lissé
                SetBrushOrgEx(hdc, 0, 0, std::ptr::null_mut());
            } else {
                SetStretchBltMode(hdc, 3); // COLORONCOLOR : copie directe
            }
            StretchDIBits(hdc, 0, 0, w, h, 0, 0, pw, ph, pm.data().as_ptr() as *const c_void, &bmi, 0, 0x00CC0020);
            ReleaseDC(hwnd, hdc);
        }
    }

    pub fn memoire() -> Option<usize> {
        let mut c = Compteurs { cb: std::mem::size_of::<Compteurs>() as u32, ..Default::default() };
        unsafe { (K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) != 0).then_some(c.travail) }
    }
}

struct App<G: Game> {
    g: G,
    #[cfg(not(windows))]
    ctx: softbuffer::Context<winit::event_loop::OwnedDisplayHandle>,
    #[cfg(not(windows))]
    surf: Option<softbuffer::Surface<winit::event_loop::OwnedDisplayHandle, Rc<Window>>>,
    win: Option<Rc<Window>>,
    glyphs: RefCell<Glyphs>,
    /// fond en cache et son empreinte
    fond: Option<Pixmap>,
    fond_sig: Option<u64>,
    frame: Option<Pixmap>,
    /// plafond de pixels de l'image interne (baisse si le garde-fou le demande)
    pixels_max: f64,
    last: Instant,
    /// instant de la dernière image affichée (rythme plafonné pendant les animations)
    last_frame: Instant,
    /// dernier contrôle de la mémoire
    controle: Instant,
    dirty: bool,
    fs: bool,
    mouse: (f64, f64),
    shot: bool,
}

impl<G: Game> App<G> {
    /// Pixels de la fenêtre -> coordonnées logiques.
    fn logical(&self, x: f64, y: f64) -> (f32, f32) {
        let Some(w) = &self.win else { return (0.0, 0.0) };
        let s = w.inner_size();
        let sc = (s.width as f32 / WW).min(s.height as f32 / WH);
        let ox = ((s.width as f32 - WW * sc) / 2.0).floor();
        let oy = ((s.height as f32 - WH * sc) / 2.0).floor();
        ((x as f32 - ox) / sc, (y as f32 - oy) / sc)
    }

    /// Taille de l'image interne pour une fenêtre w x h (plafonnée en pixels).
    fn taille_interne(&self, w: u32, h: u32) -> (u32, u32) {
        let k = (self.pixels_max / (w as f64 * h as f64)).sqrt().min(1.0);
        (((w as f64 * k).round() as u32).max(1), ((h as f64 * k).round() as u32).max(1))
    }

    fn render(&mut self) {
        let Some(win) = self.win.clone() else { return };
        let size = win.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let (iw, ih) = self.taille_interne(size.width, size.height);
        if self.fond.as_ref().map(|b| (b.width(), b.height())) != Some((iw, ih)) {
            // nouvelle taille : on libère d'abord les anciennes images (pas de pic de mémoire)
            self.fond = None;
            self.frame = None;
            self.glyphs.borrow_mut().clear();
            self.fond = Pixmap::new(iw, ih);
            self.frame = Pixmap::new(iw, ih);
            self.fond_sig = None;
        }
        let sig = self.g.fond_signature();
        if self.fond_sig != Some(sig) {
            if let Some(fond) = &mut self.fond {
                fond.fill(tiny_skia::Color::BLACK);
                let g = Gfx::fit(fond, &self.glyphs);
                self.g.background(&g);
            }
            self.fond_sig = Some(sig);
        }
        let (Some(fond), Some(frame)) = (&self.fond, &mut self.frame) else { return };
        frame.data_mut().copy_from_slice(fond.data());
        {
            let g = Gfx::fit(frame, &self.glyphs);
            self.g.draw(&g);
        }
        if self.shot {
            self.shot = false;
            let _ = frame.save_png(self.g.shot_name());
        }
        self.afficher(&win, size.width, size.height);
        self.dirty = false;
        self.last_frame = Instant::now();
    }

    #[cfg(windows)]
    fn afficher(&mut self, win: &Window, w: u32, h: u32) {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let Some(frame) = &mut self.frame else { return };
        if let Ok(RawWindowHandle::Win32(hd)) = win.window_handle().map(|h| h.as_raw()) {
            gdi::afficher(hd.hwnd.get(), frame, w as i32, h as i32);
        }
    }

    #[cfg(not(windows))]
    fn afficher(&mut self, _win: &Window, w: u32, h: u32) {
        use std::num::NonZeroU32;
        let (Some(frame), Some(surf)) = (&self.frame, &mut self.surf) else { return };
        let (Some(nw), Some(nh)) = (NonZeroU32::new(w), NonZeroU32::new(h)) else { return };
        let _ = surf.resize(nw, nh);
        let Ok(mut buf) = surf.buffer_mut() else { return };
        // agrandissement au plus proche si l'image interne est plus petite que la fenêtre
        let (fw, fh) = (frame.width() as usize, frame.height() as usize);
        let px = frame.pixels();
        for y in 0..h as usize {
            let sy = (y * fh / h as usize).min(fh - 1);
            for x in 0..w as usize {
                let s = px[sy * fw + (x * fw / w as usize).min(fw - 1)];
                buf[y * w as usize + x] = (s.red() as u32) << 16 | (s.green() as u32) << 8 | s.blue() as u32;
            }
        }
        let _ = buf.present();
    }

    /// Garde-fou : si la mémoire approche de la limite, vider les caches puis baisser la résolution interne.
    fn surveiller(&mut self) {
        if self.controle.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.controle = Instant::now();
        let Some(m) = memoire() else { return };
        if m > ALERTE_RAM {
            self.g.alleger();
            self.glyphs.borrow_mut().clear();
            self.fond_sig = None;
            if memoire().unwrap_or(0) > ALERTE_RAM {
                self.pixels_max = (self.pixels_max * 0.8).max(0.5e6);
                self.fond = None;
                self.frame = None;
            }
            self.dirty = true;
        }
    }

    fn sync_fullscreen(&mut self) {
        let want = self.g.fullscreen();
        if want != self.fs {
            self.fs = want;
            if let Some(w) = &self.win {
                w.set_fullscreen(want.then_some(Fullscreen::Borderless(None)));
            }
        }
    }
}

impl<G: Game> ApplicationHandler for App<G> {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.win.is_some() {
            return;
        }
        // 1100 x 800 si l'écran le permet, sinon réduite pour tenir (barre des tâches et bord compris)
        let (mw, mh) = el.primary_monitor().map(|m| (m.size().width as f32, m.size().height as f32)).unwrap_or((1920.0, 1080.0));
        let k = 1f32.min((mw - 40.0) / WW).min((mh - 120.0) / WH);
        let attrs = Window::default_attributes()
            .with_title(self.g.title())
            .with_inner_size(PhysicalSize::new((WW * k) as u32, (WH * k) as u32));
        let Ok(win) = el.create_window(attrs) else {
            el.exit();
            return;
        };
        let win = Rc::new(win);
        #[cfg(not(windows))]
        {
            self.surf = softbuffer::Surface::new(&self.ctx, win.clone()).ok();
        }
        self.win = Some(win);
        self.sync_fullscreen();
        self.dirty = true;
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.dirty = true,
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let k = match &event.logical_key {
                    WKey::Named(NamedKey::Escape) => Some(Key::Esc),
                    WKey::Named(NamedKey::F11) => Some(Key::F11),
                    WKey::Named(NamedKey::F12) => {
                        self.shot = true;
                        None
                    }
                    WKey::Named(NamedKey::Space) => Some(Key::Space),
                    WKey::Named(NamedKey::Enter) => Some(Key::Enter),
                    WKey::Named(NamedKey::Backspace) => Some(Key::Back),
                    _ => event.text.as_ref().and_then(|t| t.chars().next()).filter(|c| !c.is_control()).map(Key::Char),
                };
                if let Some(k) = k {
                    self.g.key(k);
                }
                self.dirty = true;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse = (position.x, position.y);
                let (x, y) = self.logical(position.x, position.y);
                if self.g.hover(x, y) {
                    self.dirty = true;
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let (x, y) = self.logical(self.mouse.0, self.mouse.1);
                self.g.click(x, y);
                self.dirty = true;
            }
            _ => {}
        }
        if self.g.quit() {
            el.exit();
        }
        self.sync_fullscreen();
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f64() * 1000.0;
        self.last = now;
        if self.g.tick(dt.min(100.0)) {
            self.dirty = true;
        }
        if self.g.quit() {
            el.exit();
            return;
        }
        self.surveiller();
        // au plus ~60 images/s : pendant une animation, on attend que l'image précédente ait 16 ms
        const FRAME: Duration = Duration::from_millis(16);
        let next_frame = self.last_frame + FRAME;
        if self.dirty && now >= next_frame {
            if let Some(w) = &self.win {
                w.request_redraw();
            }
        }
        // réveil : prochaine image d'une animation, sinon toutes les 50 ms (réseau, minuteries)
        let wake = if self.dirty || self.g.animating() { next_frame.max(now + Duration::from_millis(2)) } else { now + Duration::from_millis(50) };
        el.set_control_flow(ControlFlow::WaitUntil(wake));
    }
}

/// Ouvre la fenêtre et fait tourner le jeu jusqu'à sa fermeture.
pub fn run<G: Game>(g: G) {
    let el = match EventLoop::new() {
        Ok(el) => el,
        // Linux : si le serveur Wayland ne répond pas, on retente en X11 (XWayland)
        Err(_) if std::env::var_os("WAYLAND_DISPLAY").is_some() => {
            std::env::remove_var("WAYLAND_DISPLAY");
            match EventLoop::new() {
                Ok(el) => el,
                Err(e) => return eprintln!("Impossible d'ouvrir une fenêtre : {e}"),
            }
        }
        Err(e) => return eprintln!("Impossible d'ouvrir une fenêtre : {e}"),
    };
    #[cfg(not(windows))]
    let Ok(ctx) = softbuffer::Context::new(el.owned_display_handle()) else { return };
    let mut app = App {
        g,
        #[cfg(not(windows))]
        ctx,
        #[cfg(not(windows))]
        surf: None,
        win: None,
        glyphs: RefCell::new(Glyphs::default()),
        fond: None,
        fond_sig: None,
        frame: None,
        pixels_max: PIXELS_MAX,
        last: Instant::now(),
        last_frame: Instant::now(),
        controle: Instant::now(),
        dirty: true,
        fs: false,
        mouse: (0.0, 0.0),
        shot: false,
    };
    let _ = el.run_app(&mut app);
}
