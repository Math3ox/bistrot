//! Dessin 100 % processeur (aucune carte graphique) en coordonnées logiques 1100 x 800 : formes pleines
//! anticrénelées avec contour (tiny-skia), dégradés linéaires et circulaires, transformation affine (objets
//! posés, tournés, écrasés par la perspective), images posées avec n'importe quelle transformation, textes
//! alignés comme sous GDI (haut du texte = haut de la cellule de la police, ab_glyph), mis à l'échelle de la
//! fenêtre.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, PremultipliedColorU8, Stroke, Transform};

pub const WW: f32 = 1100.0;
pub const WH: f32 = 800.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }
}

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

/// Couleur c avec l'opacité a.
pub const fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

/// Mélange de deux couleurs (k = 0 : a, k = 1 : b).
pub fn mix(a: Color, b: Color, k: f32) -> Color {
    Color::new(a.r + (b.r - a.r) * k, a.g + (b.g - a.g) * k, a.b + (b.b - a.b) * k, a.a + (b.a - a.a) * k)
}

fn stops(arrets: &[(f32, Color)]) -> Vec<tiny_skia::GradientStop> {
    arrets
        .iter()
        .map(|&(p, c)| {
            let col = tiny_skia::Color::from_rgba(c.r.clamp(0.0, 1.0), c.g.clamp(0.0, 1.0), c.b.clamp(0.0, 1.0), c.a.clamp(0.0, 1.0));
            tiny_skia::GradientStop::new(p, col.unwrap_or(tiny_skia::Color::BLACK))
        })
        .collect()
}

/// Chemin d'un rectangle arrondi (pour les dégradés).
pub fn chemin_rrect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    Gfx::poly_path(&Gfx::rrect_pts(x, y, w, h, r))
}

/// Chemin d'une ellipse.
pub fn chemin_ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Option<tiny_skia::Path> {
    tiny_skia::Rect::from_ltrb(cx - rx, cy - ry, cx + rx, cy + ry).and_then(PathBuilder::from_oval)
}

/// Chemin d'un polygone.
pub fn chemin_poly(pts: &[(f32, f32)]) -> Option<tiny_skia::Path> {
    Gfx::poly_path(pts)
}

/// Petit générateur pseudo-aléatoire (textures du décor : toujours les mêmes à chaque dessin).
pub struct Hasard(pub u64);

impl Hasard {
    /// Nombre entre 0 et 1 (splitmix64 : bon mélange même pour une petite graine).
    pub fn f(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Nombre entre a et b.
    pub fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
}

fn paint(c: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(tiny_skia::Color::from_rgba(c.r, c.g, c.b, c.a).unwrap_or(tiny_skia::Color::BLACK));
    p.anti_alias = true;
    p
}

/// Transformation affine : (x, y) -> (a x + c y + tx, b x + d y + ty).
#[derive(Clone, Copy)]
pub struct Xf {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Xf {
    pub const ID: Xf = Xf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };

    /// Objet centré en (cx, cy), agrandi de `scale`, tourné de `angle`, puis écrasé verticalement de `sq`.
    pub fn place(cx: f64, cy: f64, scale: f64, angle: f64, sq: f64) -> Xf {
        let (s, c) = angle.sin_cos();
        Xf {
            a: (scale * c) as f32,
            b: (scale * s * sq) as f32,
            c: (-scale * s) as f32,
            d: (scale * c * sq) as f32,
            tx: cx as f32,
            ty: cy as f32,
        }
    }

    /// Objet centré en (cx, cy) agrandi de s (sans rotation).
    pub fn centre(cx: f32, cy: f32, s: f32) -> Xf {
        Xf { a: s, b: 0.0, c: 0.0, d: s, tx: cx, ty: cy }
    }

    /// Demi-tour autour de l'origine du repère (figures et indices « à double tête »).
    pub const DEMI_TOUR: Xf = Xf { a: -1.0, b: 0.0, c: 0.0, d: -1.0, tx: 0.0, ty: 0.0 };

    /// Composition : d'abord `l` (repère local), puis `self`.
    pub fn puis(&self, l: &Xf) -> Xf {
        Xf {
            a: self.a * l.a + self.c * l.b,
            b: self.b * l.a + self.d * l.b,
            c: self.a * l.c + self.c * l.d,
            d: self.b * l.c + self.d * l.d,
            tx: self.a * l.tx + self.c * l.ty + self.tx,
            ty: self.b * l.tx + self.d * l.ty + self.ty,
        }
    }

    fn ts(&self) -> Transform {
        Transform::from_row(self.a, self.b, self.c, self.d, self.tx, self.ty)
    }

    /// Facteur d'agrandissement moyen.
    pub fn scale(&self) -> f32 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }
}

static NEXT_FONT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

/// Une police à une taille donnée (taille = hauteur du cadratin, comme une hauteur négative sous GDI).
#[derive(Clone)]
pub struct Face {
    font: Option<(usize, Rc<FontVec>)>,
    pub size: f32,
    /// hauteur au-dessus de la ligne de base, en fraction de la taille (usWinAscent / unitsPerEm)
    pub asc: f32,
}

/// Lit usWinAscent / unitsPerEm dans les tables OS/2 et head d'une police TrueType.
fn win_ascent(b: &[u8]) -> Option<f32> {
    let u16at = |o: usize| b.get(o..o + 2).map(|s| u16::from_be_bytes([s[0], s[1]]));
    let u32at = |o: usize| b.get(o..o + 4).map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]));
    let n = u16at(4)? as usize;
    let (mut os2, mut head) = (None, None);
    for i in 0..n {
        let r = 12 + i * 16;
        let off = u32at(r + 8)? as usize;
        match b.get(r..r + 4)? {
            b"OS/2" => os2 = Some(off),
            b"head" => head = Some(off),
            _ => {}
        }
    }
    let upem = u16at(head? + 18)? as f32;
    let asc = u16at(os2? + 74)? as f32;
    (upem > 0.0).then(|| asc / upem)
}

/// Chemin du fichier de police choisi par fontconfig pour un motif (Linux / BSD).
#[cfg(all(unix, not(target_os = "macos")))]
fn fc_match(pattern: &str) -> Option<String> {
    let out = std::process::Command::new("fc-match").args(["-f", "%{file}", pattern]).output().ok()?;
    let path = String::from_utf8(out.stdout).ok()?;
    (out.status.success() && !path.is_empty()).then_some(path)
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
fn fc_match(_pattern: &str) -> Option<String> {
    None
}

/// Police chargée : (identifiant, police) et hauteur au-dessus de la ligne de base.
type Chargee = (Option<(usize, Rc<FontVec>)>, f32);

/// Fichier de police (le premier qui existe dans la liste), lu seulement à la première utilisation :
/// une police dont le jeu ne se sert pas ne coûte rien en mémoire.
pub struct FontFile {
    chemins: Vec<&'static str>,
    fc: &'static str,
    chargee: std::cell::OnceCell<Chargee>,
}

impl FontFile {
    /// `fc` : motif fontconfig (ex. « sans-serif:bold ») demandé à `fc-match` si aucun chemin n'existe
    /// (sous Linux, les polices installées varient d'une distribution à l'autre : Fedora n'a que Noto).
    pub fn load(paths: &[&'static str], fc: &'static str) -> FontFile {
        FontFile { chemins: paths.to_vec(), fc, chargee: std::cell::OnceCell::new() }
    }

    fn chargee(&self) -> &Chargee {
        self.chargee.get_or_init(|| {
            self.chemins
                .iter()
                .find_map(|p| Self::open(p))
                .or_else(|| Self::open(&fc_match(self.fc)?))
                .unwrap_or((None, 0.8))
        })
    }

    fn open(path: &str) -> Option<Chargee> {
        let bytes = std::fs::read(path).ok()?;
        let asc = win_ascent(&bytes).unwrap_or(0.95);
        let font = FontVec::try_from_vec(bytes).ok()?;
        let id = NEXT_FONT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Some((Some((id, Rc::new(font))), asc))
    }

    pub fn face(&self, size: f32) -> Face {
        let (font, asc) = self.chargee();
        Face { font: font.clone(), size, asc: *asc }
    }
}

/// Échelle ab_glyph pour une taille de cadratin (em) en pixels : ab_glyph mesure la hauteur
/// ascendante + descendante, GDI le cadratin.
fn em_scale(font: &FontVec, em_px: f32, squash: f32) -> PxScale {
    let k = font.height_unscaled() / font.units_per_em().unwrap_or(1000.0);
    PxScale { x: em_px * k, y: em_px * k * squash }
}

/// Alignement horizontal du texte (le point donné est toujours le haut du texte).
#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Glyphe rastérisé : décalage par rapport au point de base et couverture.
struct Glyph {
    left: i32,
    top: i32,
    w: usize,
    h: usize,
    cov: Vec<u8>,
}

/// Cache des glyphes déjà rastérisés (police, glyphe, tailles en 1/16 de pixel).
#[derive(Default)]
pub struct Glyphs(HashMap<(usize, u16, u32, u32), Option<Glyph>>);

impl Glyphs {
    /// Oublie les glyphes (changement de taille de la fenêtre).
    pub fn clear(&mut self) {
        self.0 = HashMap::new();
    }
}

/// Peintre : passage des coordonnées logiques aux pixels de l'image cible.
pub struct Gfx<'a> {
    pub sc: f32,
    pub ox: f32,
    pub oy: f32,
    pm: RefCell<&'a mut Pixmap>,
    glyphs: &'a RefCell<Glyphs>,
}

impl<'a> Gfx<'a> {
    pub fn new(pm: &'a mut Pixmap, glyphs: &'a RefCell<Glyphs>, sc: f32, ox: f32, oy: f32) -> Gfx<'a> {
        Gfx { sc, ox, oy, pm: RefCell::new(pm), glyphs }
    }

    /// Le jeu 1100 x 800 mis à l'échelle d'une image w x h, proportions gardées, centré.
    pub fn fit(pm: &'a mut Pixmap, glyphs: &'a RefCell<Glyphs>) -> Gfx<'a> {
        let (w, h) = (pm.width() as f32, pm.height() as f32);
        let sc = (w / WW).min(h / WH);
        let (ox, oy) = (((w - WW * sc) / 2.0).floor(), ((h - WH * sc) / 2.0).floor());
        Gfx::new(pm, glyphs, sc, ox, oy)
    }

    /// Copie de l'image en cours (pour la mettre en cache).
    pub fn snapshot(&self) -> Pixmap {
        self.pm.borrow().clone()
    }

    /// Taille de l'image en cours (pixels).
    pub fn snapshot_size(&self) -> (u32, u32) {
        let pm = self.pm.borrow();
        (pm.width(), pm.height())
    }

    /// Rectangle logique (x0, y0, x1, y1) couvert par toute l'image : plus large que 1100 x 800 quand les
    /// proportions de l'écran diffèrent (bandes sur les côtés en plein écran 16:9).
    pub fn visible(&self) -> (f32, f32, f32, f32) {
        let (w, h) = self.snapshot_size();
        (-self.ox / self.sc, -self.oy / self.sc, (w as f32 - self.ox) / self.sc, (h as f32 - self.oy) / self.sc)
    }

    /// Accès direct aux pixels de l'image en cours (caches de décor).
    pub fn pixels<R>(&self, f: impl FnOnce(&mut Pixmap) -> R) -> R {
        let mut pm = self.pm.borrow_mut();
        f(&mut pm)
    }

    /// Remplace l'image en cours par une image en cache de même taille.
    pub fn restore(&self, img: &Pixmap) {
        let mut pm = self.pm.borrow_mut();
        if pm.width() == img.width() && pm.height() == img.height() {
            pm.data_mut().copy_from_slice(img.data());
        }
    }

    /// Même mise à l'échelle, dans une autre image de même taille.
    pub fn on<'c>(&self, pm: &'c mut Pixmap) -> Gfx<'c>
    where
        'a: 'c,
    {
        Gfx::new(pm, self.glyphs, self.sc, self.ox, self.oy)
    }

    /// Cache des glyphes (pour dessiner dans d'autres images avec les mêmes polices).
    pub fn glyphs(&self) -> &'a RefCell<Glyphs> {
        self.glyphs
    }

    fn view(&self) -> Transform {
        Transform::from_row(self.sc, 0.0, 0.0, self.sc, self.ox, self.oy)
    }

    /// Forme quelconque (chemin tiny-skia en coordonnées de l'objet), pleine et/ou avec un contour d'épaisseur w.
    pub fn forme(&self, xf: &Xf, path: &tiny_skia::Path, fill: Option<Color>, line: Option<Color>, w: f32) {
        self.path(xf, Some(path.clone()), fill, line, w);
    }

    /// Rectangle arrondi rempli d'un dégradé vertical (haut -> bas), avec contour éventuel.
    pub fn rrect_degrade(&self, xf: &Xf, x: f32, y: f32, w: f32, h: f32, r: f32, haut: Color, bas: Color, line: Option<Color>) {
        let Some(path) = chemin_rrect(x, y, w, h, r) else { return };
        self.lineaire(xf, &path, (0.0, y), (0.0, y + h), &[(0.0, haut), (1.0, bas)]);
        if line.is_some() {
            self.path(xf, Some(path), None, line, 0.6);
        }
    }

    fn remplir_degrade(&self, xf: &Xf, path: &tiny_skia::Path, shader: Option<tiny_skia::Shader>) {
        if let Some(shader) = shader {
            let paint = Paint { shader, anti_alias: true, ..Default::default() };
            let ts = self.view().pre_concat(xf.ts());
            self.pm.borrow_mut().fill_path(path, &paint, FillRule::Winding, ts, None);
        }
    }

    /// Forme remplie d'un dégradé linéaire de p0 à p1 (repère de l'objet), arrêts (position 0..1, couleur).
    pub fn lineaire(&self, xf: &Xf, path: &tiny_skia::Path, p0: (f32, f32), p1: (f32, f32), arrets: &[(f32, Color)]) {
        let shader = tiny_skia::LinearGradient::new(
            tiny_skia::Point::from_xy(p0.0, p0.1),
            tiny_skia::Point::from_xy(p1.0, p1.1),
            stops(arrets),
            tiny_skia::SpreadMode::Pad,
            Transform::identity(),
        );
        self.remplir_degrade(xf, path, shader);
    }

    /// Forme remplie d'un dégradé circulaire de centre c et de rayon r (repère de l'objet : un xf écrasé
    /// donne une lueur en ellipse, comme une lumière posée sur la table).
    pub fn radial(&self, xf: &Xf, path: &tiny_skia::Path, c: (f32, f32), r: f32, arrets: &[(f32, Color)]) {
        let p = tiny_skia::Point::from_xy(c.0, c.1);
        let shader = tiny_skia::RadialGradient::new(p, p, r, stops(arrets), tiny_skia::SpreadMode::Pad, Transform::identity());
        self.remplir_degrade(xf, path, shader);
    }

    /// Lueur : disque de rayon r centré en c rempli d'un dégradé circulaire (transparent au bord).
    pub fn lueur(&self, xf: &Xf, c: (f32, f32), r: f32, arrets: &[(f32, Color)]) {
        if let Some(path) = chemin_ellipse(c.0, c.1, r, r) {
            self.radial(xf, &path, c, r, arrets);
        }
    }

    fn rrect_pts(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<(f32, f32)> {
        let r = r.min(w / 2.0).min(h / 2.0);
        let mut pts = Vec::with_capacity(40);
        if r <= 0.0 {
            pts.extend([(x, y), (x + w, y), (x + w, y + h), (x, y + h)]);
        } else {
            let corners = [(x + w - r, y + r, -90.0f32), (x + w - r, y + h - r, 0.0), (x + r, y + h - r, 90.0), (x + r, y + r, 180.0)];
            for (cx, cy, a0) in corners {
                for k in 0..=8 {
                    let a = (a0 + k as f32 * 11.25).to_radians();
                    pts.push((cx + r * a.cos(), cy + r * a.sin()));
                }
            }
        }
        pts
    }

    fn path(&self, xf: &Xf, path: Option<tiny_skia::Path>, fill: Option<Color>, line: Option<Color>, w: f32) {
        let Some(path) = path else { return };
        let ts = self.view().pre_concat(xf.ts());
        let mut pm = self.pm.borrow_mut();
        if let Some(c) = fill {
            pm.fill_path(&path, &paint(c), FillRule::Winding, ts, None);
        }
        if let Some(c) = line {
            // épaisseur voulue en pixels, ramenée dans le repère de l'objet
            let px = (w * self.sc * xf.scale()).max(0.8);
            let stroke = Stroke { width: px / (self.sc * xf.scale()).max(1e-3), ..Default::default() };
            pm.stroke_path(&path, &paint(c), &stroke, ts, None);
        }
    }

    fn poly_path(pts: &[(f32, f32)]) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        let (x0, y0) = *pts.first()?;
        pb.move_to(x0, y0);
        for &(x, y) in &pts[1..] {
            pb.line_to(x, y);
        }
        pb.close();
        pb.finish()
    }

    /// Polygone plein et/ou contour (convexe ou non).
    pub fn poly(&self, xf: &Xf, pts: &[(f32, f32)], fill: Option<Color>, line: Option<Color>) {
        self.path(xf, Self::poly_path(pts), fill, line, 0.6);
    }

    /// Polygone avec un contour d'épaisseur w (unités logiques).
    pub fn poly_w(&self, xf: &Xf, pts: &[(f32, f32)], fill: Option<Color>, line: Option<Color>, w: f32) {
        self.path(xf, Self::poly_path(pts), fill, line, w);
    }

    pub fn poly_any(&self, xf: &Xf, pts: &[(f32, f32)], fill: Color, line: Option<Color>) {
        self.poly(xf, pts, Some(fill), line);
    }

    /// Rectangle arrondi (rayon r).
    pub fn rrect(&self, xf: &Xf, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Option<Color>, line: Option<Color>) {
        self.poly(xf, &Self::rrect_pts(x, y, w, h, r), fill, line);
    }

    /// Rectangle arrondi avec un contour d'épaisseur lw (unités logiques).
    pub fn rrect_w(&self, xf: &Xf, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Option<Color>, line: Option<Color>, lw: f32) {
        self.poly_w(xf, &Self::rrect_pts(x, y, w, h, r), fill, line, lw);
    }

    /// Ligne brisée (non fermée) d'épaisseur w.
    pub fn polyligne(&self, xf: &Xf, pts: &[(f32, f32)], w: f32, c: Color) {
        let mut pb = PathBuilder::new();
        let Some(&(x0, y0)) = pts.first() else { return };
        pb.move_to(x0, y0);
        for &(x, y) in &pts[1..] {
            pb.line_to(x, y);
        }
        self.path(xf, pb.finish(), None, Some(c), w);
    }

    pub fn ellipse(&self, xf: &Xf, cx: f32, cy: f32, rx: f32, ry: f32, fill: Option<Color>, line: Option<Color>) {
        let rect = tiny_skia::Rect::from_ltrb(cx - rx, cy - ry, cx + rx, cy + ry);
        self.path(xf, rect.and_then(PathBuilder::from_oval), fill, line, 0.6);
    }

    pub fn line(&self, xf: &Xf, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: Color) {
        let mut pb = PathBuilder::new();
        pb.move_to(x0, y0);
        pb.line_to(x1, y1);
        self.path(xf, pb.finish(), None, Some(c), w);
    }

    /// Pose une image (repère : pixel (0, 0) de l'image en (x0, y0) de l'objet, `ppu` pixels par unité),
    /// transformée par xf.
    pub fn image(&self, xf: &Xf, img: &Pixmap, x0: f32, y0: f32, ppu: f32) {
        let ts = self.view().pre_concat(xf.ts()).pre_concat(Transform::from_row(1.0 / ppu, 0.0, 0.0, 1.0 / ppu, x0, y0));
        if ts.kx == 0.0 && ts.ky == 0.0 && (ts.sx - 1.0).abs() < 1e-3 && (ts.sy - 1.0).abs() < 1e-3 {
            // image posée à l'échelle 1:1 sans rotation : simple copie au pixel près (bien plus rapide)
            let paint = PixmapPaint { quality: tiny_skia::FilterQuality::Nearest, ..Default::default() };
            let (x, y) = (ts.tx.round() as i32, ts.ty.round() as i32);
            self.pm.borrow_mut().draw_pixmap(x, y, img.as_ref(), &paint, Transform::identity(), None);
            return;
        }
        if ts.kx == 0.0 && ts.ky == 0.0 && (ts.sx.abs() - 1.0).abs() < 1e-3 && (ts.sy.abs() - 1.0).abs() < 1e-3 {
            // retournée sans changer d'échelle (indices et figures « à double tête ») : pixels recopiés
            let paint = PixmapPaint { quality: tiny_skia::FilterQuality::Nearest, ..Default::default() };
            let t = Transform::from_row(ts.sx.signum(), 0.0, 0.0, ts.sy.signum(), ts.tx.round(), ts.ty.round());
            self.pm.borrow_mut().draw_pixmap(0, 0, img.as_ref(), &paint, t, None);
            return;
        }
        // agrandie, réduite ou tournée : filtre bicubique (plus net que le bilinéaire)
        let paint = PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() };
        self.pm.borrow_mut().draw_pixmap(0, 0, img.as_ref(), &paint, ts, None);
    }

    /// Largeur d'un texte (unités logiques).
    pub fn text_width(&self, s: &str, f: &Face) -> f32 {
        let Some((_, font)) = &f.font else { return 0.0 };
        let sf = font.as_scaled(em_scale(font, f.size * self.sc, 1.0));
        let mut w = 0.0;
        let mut prev = None;
        for ch in s.chars() {
            let id = sf.glyph_id(ch);
            if let Some(p) = prev {
                w += sf.kern(p, id);
            }
            w += sf.h_advance(id);
            prev = Some(id);
        }
        w / self.sc
    }

    /// Texte dont le haut est en y (comme TA_TOP sous GDI) ; squash = écrasement vertical (texte couché).
    pub fn text_sq(&self, s: &str, x: f32, y: f32, f: &Face, c: Color, al: Align, squash: f32) {
        let Some((fid, font)) = &f.font else { return };
        if s.is_empty() {
            return;
        }
        let px = f.size * self.sc;
        let scale = em_scale(font, px, squash);
        let sf = font.as_scaled(scale);
        let w = self.text_width(s, f) * self.sc;
        let p = (self.ox + x * self.sc, self.oy + y * self.sc);
        let mut pen = match al {
            Align::Left => p.0,
            Align::Center => p.0 - w / 2.0,
            Align::Right => p.0 - w,
        };
        let base = (p.1 + f.asc * px * squash).round() as i32;
        let key_sz = ((scale.x * 16.0) as u32, (scale.y * 16.0) as u32);
        let mut cache = self.glyphs.borrow_mut();
        let mut pm = self.pm.borrow_mut();
        let (pw, ph) = (pm.width() as i32, pm.height() as i32);
        let pixels = pm.pixels_mut();
        let mut prev = None;
        for ch in s.chars() {
            let id = sf.glyph_id(ch);
            if let Some(pv) = prev {
                pen += sf.kern(pv, id);
            }
            prev = Some(id);
            let gx = pen.round() as i32;
            pen += sf.h_advance(id);
            let g = cache.0.entry((*fid, id.0, key_sz.0, key_sz.1)).or_insert_with(|| {
                let og = font.outline_glyph(id.with_scale_and_position(scale, ab_glyph::point(0.0, 0.0)))?;
                let b = og.px_bounds();
                let (w, h) = (b.width() as usize, b.height() as usize);
                let mut cov = vec![0u8; w * h];
                og.draw(|x, y, v| {
                    if let Some(c) = cov.get_mut(y as usize * w + x as usize) {
                        *c = (v.clamp(0.0, 1.0) * 255.0) as u8;
                    }
                });
                Some(Glyph { left: b.min.x as i32, top: b.min.y as i32, w, h, cov })
            });
            let Some(g) = g else { continue };
            for gy in 0..g.h {
                let yy = base + g.top + gy as i32;
                if yy < 0 || yy >= ph {
                    continue;
                }
                for gxx in 0..g.w {
                    let xx = gx + g.left + gxx as i32;
                    let cv = g.cov[gy * g.w + gxx];
                    if cv == 0 || xx < 0 || xx >= pw {
                        continue;
                    }
                    let sa = c.a * cv as f32 / 255.0;
                    let d = &mut pixels[(yy * pw + xx) as usize];
                    let inv = 1.0 - sa;
                    let a = (sa * 255.0 + d.alpha() as f32 * inv).round().min(255.0) as u8;
                    let ch = |src: f32, dst: u8| ((src * sa * 255.0 + dst as f32 * inv).round() as u8).min(a);
                    let (r, gg, bb) = (ch(c.r, d.red()), ch(c.g, d.green()), ch(c.b, d.blue()));
                    if let Some(px) = PremultipliedColorU8::from_rgba(r, gg, bb, a) {
                        *d = px;
                    }
                }
            }
        }
    }

    pub fn text(&self, s: &str, x: f32, y: f32, f: &Face, c: Color, al: Align) {
        self.text_sq(s, x, y, f, c, al, 1.0);
    }
}
