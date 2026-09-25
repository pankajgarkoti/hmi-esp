//! Deterministic simulation and presentation; independent of audio, storage and clocks.
use crate::{BatteryTelemetry, Button, DashboardState, Gesture, Health, InputEvent};
use alloc::{format, string::String, vec, vec::Vec};
use embedded_graphics::{
    mono_font::{
        ascii::{FONT_6X10, FONT_9X15_BOLD},
        MonoTextStyle,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};

const INK: BinaryColor = BinaryColor::Off;
const PAPER: BinaryColor = BinaryColor::On;
pub const LEGACY_GRID_WIDTH: usize = 100;
pub const LEGACY_GRID_HEIGHT: usize = 62;
pub const GRID_WIDTH: usize = 200;
pub const GRID_HEIGHT: usize = 124;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Rule {
    #[default]
    Life,
    HighLife,
    Brain,
    Seeds,
    DayNight,
    Replicator,
    LifeWithoutDeath,
    Diamoeba,
    Maze,
    Anneal,
    TwoByTwo,
    Morley,
    Coral,
    Coagulations,
    Assimilation,
    Vote,
}

impl Rule {
    pub const ALL: [Self; 16] = [
        Self::Life,
        Self::HighLife,
        Self::Brain,
        Self::Seeds,
        Self::DayNight,
        Self::Replicator,
        Self::LifeWithoutDeath,
        Self::Diamoeba,
        Self::Maze,
        Self::Anneal,
        Self::TwoByTwo,
        Self::Morley,
        Self::Coral,
        Self::Coagulations,
        Self::Assimilation,
        Self::Vote,
    ];
    pub fn name(self) -> &'static str {
        [
            "CONWAY'S LIFE",
            "HIGHLIFE",
            "BRIAN'S BRAIN",
            "SEEDS",
            "DAY & NIGHT",
            "REPLICATOR",
            "LIFE WITHOUT DEATH",
            "DIAMOEBA",
            "MAZE",
            "ANNEAL",
            "2 X 2",
            "MORLEY",
            "CORAL",
            "COAGULATIONS",
            "ASSIMILATION",
            "VOTE",
        ][self as usize]
    }
    fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }
    pub fn description(self) -> &'static str {
        [
            "Gliders, oscillators and quiet islands",
            "Replicators emerge from simple rules",
            "Excitable sparks with fading trails",
            "Everything dies; everything is born",
            "Light and dark are equal citizens",
            "Parity blooms into fractal patterns",
            "Growth that never forgets",
            "Soft-edged islands in a pixel sea",
            "Corridors grow into living labyrinths",
            "Islands settle into smooth boundaries",
            "Blocks dance in reversible-looking steps",
            "A busy universe of moving creatures",
            "Branching reefs that slowly take root",
            "Dense matter gathers and separates",
            "Neighbourhoods merge into structures",
            "The local majority shapes the world",
        ][self as usize]
    }
    fn masks(self) -> (u16, u16) {
        let (birth, survive): (&[u8], &[u8]) = match self {
            Self::Life => (&[3], &[2, 3]),
            Self::HighLife => (&[3, 6], &[2, 3]),
            Self::Brain => (&[2], &[]),
            Self::Seeds => (&[2], &[]),
            Self::DayNight => (&[3, 6, 7, 8], &[3, 4, 6, 7, 8]),
            Self::Replicator => (&[1, 3, 5, 7], &[1, 3, 5, 7]),
            Self::LifeWithoutDeath => (&[3], &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            Self::Diamoeba => (&[3, 5, 6, 7, 8], &[5, 6, 7, 8]),
            Self::Maze => (&[3], &[1, 2, 3, 4, 5]),
            Self::Anneal => (&[4, 6, 7, 8], &[3, 5, 6, 7, 8]),
            Self::TwoByTwo => (&[3, 6], &[1, 2, 5]),
            Self::Morley => (&[3, 6, 8], &[2, 4, 5]),
            Self::Coral => (&[3], &[4, 5, 6, 7, 8]),
            Self::Coagulations => (&[3, 7, 8], &[2, 3, 5, 6, 7, 8]),
            Self::Assimilation => (&[3, 4, 5], &[4, 5, 6, 7]),
            Self::Vote => (&[5, 6, 7, 8], &[4, 5, 6, 7, 8]),
        };
        (
            birth.iter().fold(0, |m, n| m | 1 << n),
            survive.iter().fold(0, |m, n| m | 1 << n),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub rule: Rule,
    pub speed: u8,
    pub microphone: u8,
    pub linger: u8,
    pub tour: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rule: Rule::Life,
            speed: 1,
            microphone: 1,
            linger: 1,
            tour: 0,
        }
    }
}

impl Settings {
    pub fn interval_ms(self) -> u64 {
        [500, 250, 125, 62][self.speed.min(3) as usize]
    }
    pub fn clock_ms(self) -> u64 {
        [3000, 5000, 8000][self.linger.min(2) as usize]
    }
    pub fn encode(self) -> u32 {
        0xa2000000
            | (self.rule as u32)
            | ((self.speed as u32) << 4)
            | ((self.microphone as u32) << 8)
            | ((self.linger as u32) << 12)
            | ((self.tour as u32) << 16)
    }
    pub fn decode(value: u32) -> Self {
        let rule = value & 15;
        let speed = (value >> 4) & 15;
        let microphone = (value >> 8) & 15;
        let linger = (value >> 12) & 15;
        let version = value >> 24;
        let tour = (value >> 16) & 15;
        if ![0xa1, 0xa2].contains(&version)
            || value & 0x00f00000 != 0
            || speed > 3
            || microphone > 3
            || linger > 2
            || tour > 3
        {
            return Self::default();
        }
        Self {
            rule: Rule::ALL[rule as usize],
            speed: speed as u8,
            microphone: microphone as u8,
            linger: linger as u8,
            tour: tour as u8,
        }
    }
}

pub struct Automaton {
    width: usize,
    height: usize,
    cells: Vec<u8>,
    next: Vec<u8>,
    pub rule: Rule,
    pub generation: u64,
    random: u32,
}

impl Automaton {
    pub fn empty(width: usize, height: usize, rule: Rule, seed: u32) -> Self {
        assert!(width >= 3 && height >= 3 && width <= 400 && height <= 300);
        Self {
            width,
            height,
            cells: vec![0; width * height],
            next: vec![0; width * height],
            rule,
            generation: 0,
            random: seed.max(1),
        }
    }
    pub fn cells(&self) -> &[u8] {
        &self.cells
    }
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }
    pub fn cell(&self, x: usize, y: usize) -> u8 {
        self.cells[y * self.width + x]
    }
    pub fn set(&mut self, x: usize, y: usize, value: u8) {
        self.cells[y * self.width + x] = value;
    }
    pub fn population(&self) -> usize {
        self.cells.iter().filter(|&&v| v == 1).count()
    }
    fn random(&mut self) -> u32 {
        let mut x = self.random.max(1);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.random = x;
        x
    }
    pub fn reseed(&mut self) {
        self.generation = 0;
        // Mix random soups with sparse, long-lived discoveries. Torus wrapping
        // eventually changes their infinite-plane behaviour, which is intentional.
        if self.width >= 20 && self.height >= 20 && self.random() % 2 == 0 {
            let pattern: &[(usize, usize)] = match self.rule {
                Rule::Life => &[(1, 0), (3, 1), (0, 2), (1, 2), (4, 2), (5, 2), (6, 2)], // Acorn
                Rule::HighLife => &[
                    (2, 0),
                    (3, 0),
                    (4, 0),
                    (1, 1),
                    (4, 1),
                    (0, 2),
                    (4, 2),
                    (0, 3),
                    (3, 3),
                    (0, 4),
                    (1, 4),
                    (2, 4),
                ],
                Rule::Replicator => &[(0, 0), (1, 0), (0, 1), (1, 2), (2, 2)],
                Rule::LifeWithoutDeath | Rule::Maze => &[(1, 0), (2, 0), (0, 1), (1, 1), (1, 2)],
                _ => &[],
            };
            if !pattern.is_empty() {
                self.cells.fill(0);
                for &(x, y) in pattern {
                    self.set(self.width / 2 + x - 3, self.height / 2 + y - 2, 1);
                }
                if self.width >= GRID_WIDTH && self.height >= GRID_HEIGHT {
                    // Four distant, quiet soups give the recognizable core room
                    // to evolve while putting the newly expanded field to use.
                    let density = (self.seed_density() * 3 / 4).max(20);
                    for (left, top) in [(10, 10), (140, 10), (10, 80), (140, 80)] {
                        for y in top..top + 25 {
                            for x in left..left + 35 {
                                let live = self.random() % 100 < density;
                                self.set(x, y, u8::from(live));
                            }
                        }
                    }
                }
                return;
            }
        }
        let density = self.seed_density();
        for i in 0..self.cells.len() {
            self.cells[i] = u8::from(self.random() % 100 < density);
        }
        if self.rule == Rule::TwoByTwo {
            for y in (0..self.height - 1).step_by(2) {
                for x in (0..self.width - 1).step_by(2) {
                    let v = self.cell(x, y);
                    self.set(x + 1, y, v);
                    self.set(x, y + 1, v);
                    self.set(x + 1, y + 1, v);
                }
            }
        }
    }

    fn seed_density(&self) -> u32 {
        match self.rule {
            Rule::Diamoeba
            | Rule::Anneal
            | Rule::Vote
            | Rule::DayNight
            | Rule::Coral
            | Rule::Assimilation => 48,
            Rule::Seeds | Rule::Replicator => 12,
            _ => 28,
        }
    }

    /// Keep the old world centered at the same cell scale. Give its new outer
    /// territory a sparse, deterministic soup so it can use the expanded field
    /// immediately without discarding the old cells or generation counter.
    pub fn into_display_grid(self) -> Option<Self> {
        if self.dimensions() == (GRID_WIDTH, GRID_HEIGHT) {
            return Some(self);
        }
        if self.dimensions() != (LEGACY_GRID_WIDTH, LEGACY_GRID_HEIGHT) {
            return None;
        }
        let mut expanded = Self::empty(GRID_WIDTH, GRID_HEIGHT, self.rule, self.random);
        expanded.generation = self.generation;
        let left = (GRID_WIDTH - self.width) / 2;
        let top = (GRID_HEIGHT - self.height) / 2;
        let density = expanded.seed_density() * 3 / 4;
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                let value =
                    if x >= left && x < left + self.width && y >= top && y < top + self.height {
                        self.cell(x - left, y - top)
                    } else {
                        u8::from(expanded.random() % 100 < density)
                    };
                expanded.set(x, y, value);
            }
        }
        Some(expanded)
    }
    pub fn step(&mut self) {
        let (birth, survive) = self.rule.masks();
        for y in 0..self.height {
            let rows = [
                (y + self.height - 1) % self.height,
                y,
                (y + 1) % self.height,
            ];
            for x in 0..self.width {
                let cols = [(x + self.width - 1) % self.width, x, (x + 1) % self.width];
                let mut n = 0;
                for (dy, &row) in rows.iter().enumerate() {
                    for (dx, &col) in cols.iter().enumerate() {
                        if (dy != 1 || dx != 1) && self.cell(col, row) == 1 {
                            n += 1;
                        }
                    }
                }
                let current = self.cell(x, y);
                self.next[y * self.width + x] = match self.rule {
                    Rule::Brain => match current {
                        1 => 2,
                        2 => 0,
                        _ => u8::from(n == 2),
                    },
                    _ => u8::from((if current == 1 { survive } else { birth }) & (1 << n) != 0),
                };
            }
        }
        core::mem::swap(&mut self.cells, &mut self.next);
        self.generation = self.generation.saturating_add(1);
    }
    fn inject(&mut self, count: usize) {
        for _ in 0..count {
            let i = self.random() as usize % self.cells.len();
            self.cells[i] = 1;
        }
    }

    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(26 + self.cells.len());
        out.extend_from_slice(b"LIFE");
        out.extend([1, self.rule as u8]);
        out.extend((self.width as u16).to_le_bytes());
        out.extend((self.height as u16).to_le_bytes());
        out.extend(self.generation.to_le_bytes());
        out.extend(self.random.to_le_bytes());
        out.extend(&self.cells);
        let crc = checksum(&out);
        out.extend(crc.to_le_bytes());
        out
    }
    pub fn restore(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 26 || &bytes[..4] != b"LIFE" || bytes[4] != 1 || bytes[5] >= 16 {
            return None;
        }
        let width = u16::from_le_bytes(bytes[6..8].try_into().ok()?) as usize;
        let height = u16::from_le_bytes(bytes[8..10].try_into().ok()?) as usize;
        if !(3..=400).contains(&width)
            || !(3..=300).contains(&height)
            || bytes.len() != 26 + width * height
        {
            return None;
        }
        let n = bytes.len() - 4;
        if checksum(&bytes[..n]) != u32::from_le_bytes(bytes[n..].try_into().ok()?) {
            return None;
        }
        let rule = Rule::ALL[bytes[5] as usize];
        if bytes[22..n]
            .iter()
            .any(|&c| c > if rule == Rule::Brain { 2 } else { 1 })
        {
            return None;
        }
        let mut a = Self::empty(
            width,
            height,
            rule,
            u32::from_le_bytes(bytes[18..22].try_into().ok()?),
        );
        a.generation = u64::from_le_bytes(bytes[10..18].try_into().ok()?);
        a.cells.copy_from_slice(&bytes[22..n]);
        Some(a)
    }
}

fn checksum(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320u32 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Life,
    Clock,
    Settings,
    WifiSetup,
}

pub struct LivingDisplay {
    pub automaton: Automaton,
    pub settings: Settings,
    pub selected: usize,
    pub voice_ready: bool,
    pub mic_peak: u16,
    pub learn_requested: bool,
    pub sound_message: &'static str,
    pub sd_status: &'static str,
    pub wifi_setup_requested: bool,
    pub wifi_setup_active: bool,
    pub wifi_name: String,
    pub wifi_password: String,
    pub wifi_address: String,
    pub wifi_message: &'static str,
    wifi_setup_until: u64,
    parked: Vec<Option<Automaton>>,
    last_tour: u64,
    last_step: u64,
    clock_until: u64,
    voice_after: u64,
    settings_until: u64,
    audio_pending: bool,
}

impl LivingDisplay {
    pub fn new(settings: Settings, seed: u32) -> Self {
        let settings = Settings::decode(settings.encode());
        let mut automaton = Automaton::empty(GRID_WIDTH, GRID_HEIGHT, settings.rule, seed);
        automaton.reseed();
        Self {
            automaton,
            settings,
            selected: 0,
            voice_ready: false,
            mic_peak: 0,
            learn_requested: false,
            sound_message: "SNAP / SAY TIME",
            sd_status: "RAM ONLY",
            wifi_setup_requested: false,
            wifi_setup_active: false,
            wifi_name: String::new(),
            wifi_password: String::new(),
            wifi_address: String::new(),
            wifi_message: "WAITING FOR PHONE",
            wifi_setup_until: 0,
            parked: (0..16).map(|_| None).collect(),
            last_tour: 0,
            last_step: 0,
            clock_until: 0,
            voice_after: 0,
            settings_until: 0,
            audio_pending: false,
        }
    }
    pub fn switch_rule(&mut self, rule: Rule) {
        if self.automaton.rule == rule {
            return;
        }
        let index = self.automaton.rule as usize;
        let next = self.parked[rule as usize].take().unwrap_or_else(|| {
            let mut a = Automaton::empty(
                GRID_WIDTH,
                GRID_HEIGHT,
                rule,
                self.automaton.random ^ (rule as u32 + 1),
            );
            a.reseed();
            a
        });
        self.parked[index] = Some(core::mem::replace(&mut self.automaton, next));
        self.settings.rule = rule;
    }
    pub fn restore_world(&mut self, world: Automaton) -> bool {
        let Some(world) = world.into_display_grid() else {
            return false;
        };
        let rule = world.rule;
        if rule == self.settings.rule {
            self.automaton = world;
        } else {
            self.parked[rule as usize] = Some(world);
        }
        true
    }
    pub fn world(&self, rule: Rule) -> Option<&Automaton> {
        if self.automaton.rule == rule {
            Some(&self.automaton)
        } else {
            self.parked[rule as usize].as_ref()
        }
    }
    pub fn surface(&self, now: u64) -> Surface {
        if self.wifi_setup_active && now < self.wifi_setup_until {
            Surface::WifiSetup
        } else if now < self.clock_until {
            Surface::Clock
        } else if now < self.settings_until {
            Surface::Settings
        } else {
            Surface::Life
        }
    }
    pub fn show_clock(&mut self, now: u64) {
        self.settings_until = 0;
        self.clock_until = now.saturating_add(self.settings.clock_ms());
    }
    pub fn voice_time(&mut self, now: u64) -> bool {
        if self.wifi_setup_active
            || self.surface(now) == Surface::Settings
            || now < self.voice_after
        {
            return false;
        }
        self.show_clock(now);
        self.voice_after = self.clock_until.saturating_add(1500);
        true
    }
    pub fn begin_wifi_setup(&mut self, now: u64, ssid: &str, password: &str, address: &str) {
        self.wifi_setup_requested = false;
        self.wifi_setup_active = true;
        self.wifi_setup_until = now.saturating_add(180_000);
        self.clock_until = 0;
        self.wifi_name = ssid.into();
        self.wifi_password = password.into();
        self.wifi_address = address.into();
        self.wifi_message = "JOIN NETWORK, OPEN PAGE";
    }
    pub fn end_wifi_setup(&mut self, now: u64) {
        self.wifi_setup_active = false;
        self.wifi_setup_requested = false;
        self.wifi_password.clear();
        self.settings_until = now.saturating_add(20_000);
    }
    pub fn wifi_setup_expired(&self, now: u64) -> bool {
        self.wifi_setup_active && now >= self.wifi_setup_until
    }
    /// Returns true only when a persistent setting changed.
    pub fn input(&mut self, event: InputEvent, now: u64) -> bool {
        let before = self.settings;
        if self.wifi_setup_active {
            if event.button == Button::Key {
                self.end_wifi_setup(now);
            }
            return false;
        }
        match (event.button, event.gesture) {
            (Button::Key, Gesture::LongPress) => {
                self.clock_until = 0;
                self.settings_until = 0;
            }
            (Button::Key, Gesture::Click) => {
                if self.surface(now) != Surface::Life {
                    self.clock_until = 0;
                    self.settings_until = 0;
                } else {
                    self.show_clock(now);
                }
            }
            (Button::Boot, gesture) => {
                let was_settings = self.surface(now) == Surface::Settings;
                self.clock_until = 0;
                self.settings_until = now.saturating_add(20000);
                if !was_settings {
                    self.selected = 0;
                } else if gesture == Gesture::Click {
                    self.selected = (self.selected + 1) % 8;
                } else {
                    match self.selected {
                        0 => {
                            self.switch_rule(self.settings.rule.next());
                        }
                        1 => self.settings.speed = (self.settings.speed + 1) % 4,
                        2 => self.settings.microphone = (self.settings.microphone + 1) % 4,
                        3 => self.settings.linger = (self.settings.linger + 1) % 3,
                        4 => {
                            self.settings.tour = (self.settings.tour + 1) % 4;
                            self.last_tour = now;
                        }
                        5 => self.automaton.reseed(),
                        6 => {
                            self.learn_requested = true;
                            self.settings_until = now.saturating_add(20000);
                        }
                        _ => self.wifi_setup_requested = true,
                    }
                }
            }
        }
        before != self.settings
    }
    pub fn audio(&mut self, mono: &[i16]) {
        if mono.is_empty() {
            return;
        }
        self.mic_peak = 0;
        for &sample in mono {
            self.mic_peak = self.mic_peak.max(sample.unsigned_abs());
            self.automaton.random = self.automaton.random.rotate_left(5)
                ^ (sample as u16 as u32).wrapping_mul(0x45d9f3b);
        }
        self.audio_pending = true;
    }
    pub fn advance(&mut self, now: u64) {
        let tour_ms = [0, 30_000, 120_000, 300_000][self.settings.tour as usize];
        if tour_ms > 0
            && now.saturating_sub(self.last_tour) >= tour_ms
            && self.surface(now) == Surface::Life
        {
            self.switch_rule(self.settings.rule.next());
            self.last_tour = now;
        }
        let interval = self.settings.interval_ms();
        // A bounded catch-up prevents a disconnected peripheral from causing an unbounded stall.
        let steps = (now.saturating_sub(self.last_step) / interval).min(8);
        for _ in 0..steps {
            self.automaton.step();
            self.last_step = self.last_step.saturating_add(interval);
        }
        if steps == 8 {
            self.last_step = now;
        }
        if steps > 0 && self.audio_pending {
            let influence = [0, 1, 4, 12][self.settings.microphone.min(3) as usize];
            let volume = 1 + (self.mic_peak as usize / 4096).min(4);
            self.automaton.inject(influence * volume);
            self.audio_pending = false;
        }
    }
}

fn text<D: DrawTarget<Color = BinaryColor>>(
    d: &mut D,
    s: &str,
    x: i32,
    y: i32,
    bold: bool,
) -> Result<(), D::Error> {
    Text::with_baseline(
        s,
        Point::new(x, y),
        MonoTextStyle::new(if bold { &FONT_9X15_BOLD } else { &FONT_6X10 }, INK),
        Baseline::Top,
    )
    .draw(d)?;
    Ok(())
}

fn rule_line<D: DrawTarget<Color = BinaryColor>>(d: &mut D, y: i32) -> Result<(), D::Error> {
    Line::new(Point::new(12, y), Point::new(388, y))
        .into_styled(PrimitiveStyle::with_stroke(INK, 1))
        .draw(d)
}

pub fn battery_label(battery: &BatteryTelemetry) -> String {
    if battery.health == Health::Ok {
        format!("BAT {}%", battery.percent)
    } else {
        "BAT --%".into()
    }
}

pub fn render<D: DrawTarget<Color = BinaryColor> + OriginDimensions>(
    d: &mut D,
    app: &LivingDisplay,
    state: &DashboardState,
    now: u64,
) -> Result<(), D::Error> {
    d.clear(PAPER)?;
    match app.surface(now) {
        Surface::Life => {
            text(d, "LIVING", 12, 8, false)?;
            text(d, app.settings.rule.name(), 72, 8, false)?;
            text(d, &battery_label(&state.battery), 330, 8, false)?;
            for y in 0..GRID_HEIGHT {
                for x in 0..GRID_WIDTH {
                    match app.automaton.cell(x, y) {
                        1 => Rectangle::new(
                            Point::new((x * 2) as i32, 26 + (y * 2) as i32),
                            Size::new(2, 2),
                        )
                        .into_styled(PrimitiveStyle::with_fill(INK))
                        .draw(d)?,
                        2 => {
                            Pixel(Point::new((x * 2) as i32, 26 + (y * 2) as i32), INK).draw(d)?;
                        }
                        _ => {}
                    }
                }
            }
            rule_line(d, 280)?;
            text(
                d,
                &format!("GEN {:06}", app.automaton.generation),
                12,
                287,
                false,
            )?;
            text(
                d,
                if app.settings.microphone == 0 {
                    "MIC OFF"
                } else {
                    "SOUND -> LIFE"
                },
                144,
                287,
                false,
            )?;
            text(d, "BOOT: SETTINGS", 304, 287, false)?;
        }
        Surface::Clock => {
            text(d, "A MOMENT IN TIME", 12, 12, false)?;
            text(d, state.clock.zone, 269, 12, false)?;
            text(d, &battery_label(&state.battery), 330, 12, false)?;
            rule_line(d, 34)?;
            if state.clock.health == Health::Ok {
                crate::draw_large_clock(d, 76, 65, state.clock.hour, state.clock.minute)?;
                text(d, &format!("{:02}", state.clock.second), 308, 119, true)?;
                let date = format!(
                    "{}   {:02} {} {}",
                    crate::weekday_name(state.clock.weekday),
                    state.clock.day,
                    crate::month_name(state.clock.month),
                    state.clock.year
                );
                text(d, &date, (400 - date.len() as i32 * 6) / 2, 167, false)?;
            } else {
                text(d, "WAITING FOR NETWORK TIME", 92, 94, true)?;
                text(d, "The world keeps evolving.", 128, 135, false)?;
            }
            rule_line(d, 201)?;
            let env = if state.environment.health == Health::Ok {
                format!(
                    "{}.{:01} C    {}% RH",
                    state.environment.temperature_centi_c / 100,
                    state.environment.temperature_centi_c.unsigned_abs() / 10 % 10,
                    state.environment.humidity_centi_pct / 100
                )
            } else {
                "SENSOR UNAVAILABLE".into()
            };
            text(d, &env, 16, 218, false)?;
            text(
                d,
                if state.wifi.health == Health::Ok {
                    "ONLINE"
                } else {
                    "OFFLINE"
                },
                280,
                218,
                false,
            )?;
            text(
                d,
                &format!("LIFE CONTINUES  /  GEN {}", app.automaton.generation),
                16,
                262,
                false,
            )?;
            let remain = app.clock_until.saturating_sub(now);
            let width = (376 * remain / app.settings.clock_ms()).min(376) as u32;
            Rectangle::new(Point::new(12, 286), Size::new(width, 2))
                .into_styled(PrimitiveStyle::with_fill(INK))
                .draw(d)?;
        }
        Surface::Settings => {
            text(d, "SETTINGS", 16, 13, true)?;
            text(d, "BOOT NEXT / HOLD CHANGE", 250, 17, false)?;
            rule_line(d, 40)?;
            let values = [
                app.settings.rule.name().into(),
                format!("{} GEN / SEC", [2, 4, 8, 16][app.settings.speed as usize]),
                ["OFF", "SUBTLE", "RESPONSIVE", "WILD"][app.settings.microphone as usize].into(),
                format!("{} SECONDS", app.settings.clock_ms() / 1000),
                ["OFF", "30 SECONDS", "2 MINUTES", "5 MINUTES"][app.settings.tour as usize].into(),
                "HOLD TO RESEED".into(),
                "HOLD THEN SAY TIME".into(),
                "HOLD TO SET UP".into(),
            ];
            for (i, label) in [
                "AUTOMATON",
                "PACE",
                "MICROPHONE",
                "CLOCK GLANCE",
                "AUTO TOUR",
                "NEW SEED",
                "TEACH TIME",
                "WI-FI SETUP",
            ]
            .iter()
            .enumerate()
            {
                let y = 52 + i as i32 * 25;
                if i == app.selected {
                    Rectangle::new(Point::new(10, y - 4), Size::new(380, 25))
                        .into_styled(PrimitiveStyle::with_stroke(INK, 1))
                        .draw(d)?;
                    Rectangle::new(Point::new(10, y - 4), Size::new(4, 25))
                        .into_styled(PrimitiveStyle::with_fill(INK))
                        .draw(d)?;
                }
                text(d, label, 24, y + 3, false)?;
                text(d, &values[i], 191, y, values[i].len() <= 19)?;
            }
            rule_line(d, 256)?;
            text(
                d,
                if app.selected == 0 {
                    app.settings.rule.description()
                } else if app.selected == 7 {
                    app.wifi_message
                } else {
                    app.sound_message
                },
                16,
                265,
                false,
            )?;
            text(
                d,
                &format!("KEY: BACK    {}", app.sd_status),
                16,
                284,
                false,
            )?;
        }
        Surface::WifiSetup => {
            text(d, "WI-FI SETUP", 16, 12, true)?;
            text(d, "KEY TO CANCEL", 292, 17, false)?;
            rule_line(d, 39)?;
            text(d, "1   ON YOUR PHONE, JOIN", 22, 56, false)?;
            text(d, &app.wifi_name, 33, 74, true)?;
            text(d, "NETWORK PASSWORD", 33, 102, false)?;
            text(d, &app.wifi_password, 33, 119, true)?;
            rule_line(d, 153)?;
            text(d, "2   OPEN THIS PAGE IN A BROWSER", 22, 170, false)?;
            text(d, &format!("http://{}", app.wifi_address), 33, 190, true)?;
            text(d, "3   ENTER YOUR 2.4 GHz WI-FI DETAILS", 22, 225, false)?;
            rule_line(d, 255)?;
            text(d, app.wifi_message, 22, 267, false)?;
        }
    }
    Ok(())
}
