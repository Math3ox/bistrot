//! Fenêtre et boucle du jeu, sans carte graphique : winit pour la fenêtre et les entrées, softbuffer pour
//! afficher l'image calculée par le processeur (GDI sous Windows, X11 / Wayland sous Linux).
//! Le décor fixe est dessiné une fois dans une image en cache (refait seulement si la taille change) ;
//! chaque image recopie ce décor puis dessine ce qui bouge. Rien n'est redessiné quand rien ne change.

use crate::gfx::{Gfx, Glyphs, WH, WW};
use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tiny_skia::Pixmap;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{Key as WKey, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

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
    /// Décor fixe (mis en cache).
    fn background(&mut self, g: &Gfx);
    /// Tout ce qui se dessine par-dessus le décor.
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
}

struct App<G: Game> {
    g: G,
    ctx: softbuffer::Context<OwnedDisplayHandle>,
    win: Option<Rc<Window>>,
    surf: Option<softbuffer::Surface<OwnedDisplayHandle, Rc<Window>>>,
    glyphs: RefCell<Glyphs>,
    bg: Option<Pixmap>,
    frame: Option<Pixmap>,
    last: Instant,
    /// instant de la dernière image affichée (rythme plafonné pendant les animations)
    last_frame: Instant,
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

    fn render(&mut self) {
        let (Some(win), Some(surf)) = (&self.win, &mut self.surf) else { return };
        let size = win.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        // décor : refait seulement si la taille a changé
        if self.bg.as_ref().map(|b| (b.width(), b.height())) != Some((w.get(), h.get())) {
            self.glyphs.borrow_mut().clear();
            let mut bg = Pixmap::new(w.get(), h.get()).unwrap();
            bg.fill(tiny_skia::Color::BLACK);
            {
                let g = Gfx::fit(&mut bg, &self.glyphs);
                self.g.background(&g);
            }
            self.bg = Some(bg);
            self.frame = Pixmap::new(w.get(), h.get());
            let _ = surf.resize(w, h);
        }
        let (Some(bg), Some(frame)) = (&self.bg, &mut self.frame) else { return };
        frame.data_mut().copy_from_slice(bg.data());
        {
            let g = Gfx::fit(frame, &self.glyphs);
            self.g.draw(&g);
        }
        if self.shot {
            self.shot = false;
            let _ = frame.save_png(self.g.shot_name());
        }
        let Ok(mut buf) = surf.buffer_mut() else { return };
        for (d, s) in buf.iter_mut().zip(frame.pixels()) {
            *d = (s.red() as u32) << 16 | (s.green() as u32) << 8 | s.blue() as u32;
        }
        let _ = buf.present();
        self.dirty = false;
        self.last_frame = Instant::now();
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
        self.surf = softbuffer::Surface::new(&self.ctx, win.clone()).ok();
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
    let Ok(ctx) = softbuffer::Context::new(el.owned_display_handle()) else { return };
    let mut app = App {
        g,
        ctx,
        win: None,
        surf: None,
        glyphs: RefCell::new(Glyphs::default()),
        bg: None,
        frame: None,
        last: Instant::now(),
        last_frame: Instant::now(),
        dirty: true,
        fs: false,
        mouse: (0.0, 0.0),
        shot: false,
    };
    let _ = el.run_app(&mut app);
}
