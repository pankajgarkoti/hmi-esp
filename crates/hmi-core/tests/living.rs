use hmi_core::living::{Automaton, LivingDisplay, Rule, Settings, Surface};
use hmi_core::{Button, Gesture, InputEvent};

fn event(button: Button, gesture: Gesture) -> InputEvent {
    InputEvent {
        button,
        gesture,
        held_ms: 700,
    }
}

#[test]
fn conway_blinker_oscillates_without_changing_population() {
    let mut a = Automaton::empty(12, 10, Rule::Life, 42);
    for x in 4..7 {
        a.set(x, 5, 1);
    }
    a.step();
    assert_eq!(a.population(), 3);
    for y in 4..7 {
        assert_eq!(a.cell(5, y), 1);
    }
    a.step();
    for x in 4..7 {
        assert_eq!(a.cell(x, 5), 1);
    }
}

#[test]
fn toroidal_edges_are_neighbours() {
    let mut a = Automaton::empty(10, 10, Rule::Life, 1);
    for x in [9, 0, 1] {
        a.set(x, 0, 1);
    }
    a.step();
    for y in [9, 0, 1] {
        assert_eq!(a.cell(0, y), 1);
    }
    assert_eq!(a.population(), 3);
}

#[test]
fn highlife_births_with_six_neighbours_and_brain_has_refractory_state() {
    let mut a = Automaton::empty(10, 10, Rule::HighLife, 1);
    for (x, y) in [(4, 4), (5, 4), (6, 4), (4, 5), (6, 5), (4, 6)] {
        a.set(x, y, 1);
    }
    a.step();
    assert_eq!(a.cell(5, 5), 1);
    let mut b = Automaton::empty(10, 10, Rule::Brain, 1);
    b.set(4, 5, 1);
    b.set(6, 5, 1);
    b.step();
    assert_eq!(b.cell(5, 5), 1);
    assert_eq!(b.cell(4, 5), 2);
    b.step();
    assert_eq!(b.cell(4, 5), 0);
}

#[test]
fn clock_overlay_expires_while_generations_continue() {
    let mut app = LivingDisplay::new(Settings::default(), 7);
    app.advance(1000);
    let before = app.automaton.generation;
    app.show_clock(1000);
    assert_eq!(app.surface(1001), Surface::Clock);
    for ms in (1125..=6000).step_by(125) {
        app.advance(ms);
    }
    assert!(app.automaton.generation >= before + 20);
    assert_eq!(app.surface(5999), Surface::Clock);
    assert_eq!(app.surface(6000), Surface::Life);
}

#[test]
fn settings_dont_pause_simulation_and_key_returns_to_life() {
    let mut app = LivingDisplay::new(Settings::default(), 7);
    app.input(event(Button::Boot, Gesture::Click), 0);
    assert_eq!(app.surface(0), Surface::Settings);
    app.advance(250);
    assert_eq!(app.automaton.generation, 1);
    app.input(event(Button::Boot, Gesture::LongPress), 126);
    assert_eq!(app.settings.rule, Rule::HighLife);
    app.input(event(Button::Key, Gesture::Click), 127);
    assert_eq!(app.surface(127), Surface::Life);
}

#[test]
fn audio_randomness_is_replayable_and_disabled_injection_changes_no_cells() {
    let mut a = LivingDisplay::new(Settings::default(), 123);
    let mut b = LivingDisplay::new(Settings::default(), 123);
    let mut c = LivingDisplay::new(Settings::default(), 123);
    for ms in (125..=2500).step_by(125) {
        a.audio(&[123, -500, 15000, -1024]);
        b.audio(&[123, -500, 15000, -1024]);
        c.audio(&[-99, 50, 4096, -1025]);
        a.advance(ms);
        b.advance(ms);
        c.advance(ms);
    }
    assert_eq!(a.automaton.cells(), b.automaton.cells());
    assert_ne!(a.automaton.cells(), c.automaton.cells());
    let config = Settings {
        microphone: 0,
        ..Settings::default()
    };
    let mut off = LivingDisplay::new(config, 99);
    let mut quiet = LivingDisplay::new(config, 99);
    off.audio(&[10000, -25000]);
    off.advance(125);
    quiet.advance(125);
    assert_eq!(off.automaton.cells(), quiet.automaton.cells());
}

#[test]
fn settings_roundtrip_and_corrupt_storage_is_rejected() {
    let config = Settings {
        rule: Rule::Brain,
        speed: 3,
        microphone: 2,
        linger: 2,
        tour: 0,
    };
    assert_eq!(Settings::decode(config.encode()), config);
    for bad in [0, u32::MAX, 0xa100ffff] {
        assert_eq!(Settings::decode(bad), Settings::default());
    }
}

#[test]
fn changing_automata_restores_their_independent_worlds() {
    let mut app = LivingDisplay::new(Settings::default(), 7);
    app.advance(1000);
    let life = app.automaton.snapshot();
    app.switch_rule(Rule::Brain);
    app.advance(1250);
    let brain = app.automaton.snapshot();
    app.switch_rule(Rule::Life);
    assert_eq!(app.automaton.snapshot(), life);
    app.switch_rule(Rule::Brain);
    assert_eq!(app.automaton.snapshot(), brain);
}

#[test]
fn sd_snapshot_restores_cells_generation_and_randomness_and_rejects_corruption() {
    let mut app = LivingDisplay::new(Settings::default(), 19);
    app.audio(&[234, -432, 321]);
    app.advance(1000);
    let snapshot = app.automaton.snapshot();
    let mut restored = Automaton::restore(&snapshot).unwrap();
    assert_eq!(restored.snapshot(), snapshot);
    restored.step();
    app.automaton.step();
    assert_eq!(restored.snapshot(), app.automaton.snapshot());
    for at in [0, 4, 8, 15, snapshot.len() - 5, snapshot.len() - 1] {
        let mut bad = snapshot.clone();
        bad[at] ^= 0x80;
        assert!(Automaton::restore(&bad).is_none());
    }
    assert!(Automaton::restore(&snapshot[..snapshot.len() - 1]).is_none());
}

#[test]
fn all_sixteen_rules_stay_bounded_and_roundtrip() {
    assert_eq!(Rule::ALL.len(), 16);
    for rule in Rule::ALL {
        let config = Settings {
            rule,
            ..Settings::default()
        };
        assert_eq!(Settings::decode(config.encode()), config);
        let mut a = Automaton::empty(14, 10, rule, 14);
        a.reseed();
        for _ in 0..50 {
            a.step();
        }
        assert_eq!(a.generation, 50);
        assert!(a
            .cells()
            .iter()
            .all(|&c| c <= if rule == Rule::Brain { 2 } else { 1 }));
    }
}

#[test]
fn highlife_replicator_doubles_in_twelve_generations() {
    let mut a = Automaton::empty(50, 50, Rule::HighLife, 3);
    for (x, y) in [
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
    ] {
        a.set(x + 20, y + 20, 1);
    }
    assert_eq!(a.population(), 12);
    for _ in 0..12 {
        a.step();
    }
    assert_eq!(a.population(), 24);
}

#[test]
fn tour_returns_to_saved_world_without_resetting() {
    let mut app = LivingDisplay::new(
        Settings {
            tour: 1,
            microphone: 0,
            ..Settings::default()
        },
        3,
    );
    for now in (250..30000).step_by(250) {
        app.advance(now);
    }
    let generation = app.automaton.generation;
    app.advance(30000);
    assert_eq!(app.settings.rule, Rule::HighLife);
    app.switch_rule(Rule::Life);
    assert_eq!(app.automaton.generation, generation);
}

#[test]
fn wifi_setup_is_only_requested_from_settings_and_simulation_keeps_stepping() {
    let mut app = LivingDisplay::new(Settings::default(), 17);
    for now in (100..=150).step_by(10) {
        app.advance(now);
    }
    app.input(event(Button::Boot, Gesture::Click), 150);
    for index in 0..7 {
        app.input(event(Button::Boot, Gesture::Click), 160 + index * 10);
    }
    assert_eq!(app.selected, 7);
    app.input(event(Button::Boot, Gesture::LongPress), 240);
    assert!(app.wifi_setup_requested);
    app.begin_wifi_setup(240, "Living-265C", "ABCDEFGH2345", "192.168.71.1");
    assert_eq!(app.surface(250), Surface::WifiSetup);
    assert!(!app.voice_time(250));
    app.advance(1500);
    assert!(app.automaton.generation > 0);
    app.input(event(Button::Key, Gesture::Click), 1500);
    assert_ne!(app.surface(1500), Surface::WifiSetup);
}

#[test]
fn battery_label_shows_real_percentage_or_unavailable() {
    use hmi_core::{BatteryTelemetry, Health};
    let mut battery = BatteryTelemetry::default();
    assert_eq!(hmi_core::living::battery_label(&battery), "BAT --%");
    battery.health = Health::Ok;
    battery.percent = 87;
    assert_eq!(hmi_core::living::battery_label(&battery), "BAT 87%");
}

#[test]
fn voice_retrigger_cannot_pin_the_clock_forever() {
    let mut app = LivingDisplay::new(Settings::default(), 5);
    assert!(app.voice_time(1000));
    assert!(!app.voice_time(4000));
    assert_eq!(app.surface(6000), Surface::Life);
    assert!(!app.voice_time(6500));
    assert!(app.voice_time(8000));
}

#[test]
fn a_sound_cue_cannot_interrupt_settings_navigation() {
    let mut app = LivingDisplay::new(Settings::default(), 13);
    app.input(event(Button::Boot, Gesture::Click), 100);
    assert_eq!(app.surface(200), Surface::Settings);
    assert!(!app.voice_time(200));
    assert_eq!(app.surface(200), Surface::Settings);
    for step in 0..7 {
        app.input(event(Button::Boot, Gesture::Click), 210 + step * 10);
    }
    assert_eq!(app.selected, 7);
}
