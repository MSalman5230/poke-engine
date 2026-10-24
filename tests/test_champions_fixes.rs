#![cfg(not(any(feature = "gen1", feature = "gen2", feature = "gen3")))]

#[cfg(feature = "champions")]
mod champions {
    use poke_engine::choices::{Choices, MoveCategory, MOVES};
    use poke_engine::engine::abilities::Abilities;
    use poke_engine::engine::generate_instructions::{
        calculate_damage_rolls, generate_instructions_from_move_pair,
    };
    use poke_engine::engine::items::Items;
    use poke_engine::engine::state::MoveChoice;
    use poke_engine::instruction::{Instruction, StateInstructions};
    use poke_engine::pokemon::PokemonName;
    use poke_engine::state::{PokemonMoveIndex, PokemonType, SideReference, State};
    use std::str::FromStr;

    fn neutral_state() -> State {
        let mut state = State::default();
        for side in [&mut state.side_one, &mut state.side_two] {
            let pokemon = side.get_active();
            pokemon.id = PokemonName::MEW;
            pokemon.level = 50;
            pokemon.types = (PokemonType::TYPELESS, PokemonType::TYPELESS);
            pokemon.base_types = pokemon.types;
            pokemon.hp = 1000;
            pokemon.maxhp = 1000;
            pokemon.evs = (0, 0, 0, 0, 0, 0);
            pokemon.replace_move(PokemonMoveIndex::M0, Choices::SPLASH);
        }
        state.side_one.get_active().speed = 200;
        state.side_two.get_active().speed = 100;
        state
    }

    fn generate(
        state: &mut State,
        side_one: MoveChoice,
        side_two: MoveChoice,
    ) -> Vec<StateInstructions> {
        state.set_conditional_mechanics();
        let before = format!("{:?}", state);
        let result = generate_instructions_from_move_pair(state, &side_one, &side_two, false);
        assert_eq!(
            before,
            format!("{:?}", state),
            "input state was not restored"
        );
        assert!((result.iter().map(|x| x.percentage).sum::<f32>() - 100.0).abs() < 0.001);
        result
    }

    fn use_move(state: &mut State, move_id: Choices) -> Vec<StateInstructions> {
        state
            .side_one
            .get_active()
            .replace_move(PokemonMoveIndex::M0, move_id);
        generate(
            state,
            MoveChoice::Move(PokemonMoveIndex::M0),
            MoveChoice::None,
        )
    }

    fn damage_to(outcome: &StateInstructions, side_ref: SideReference) -> i16 {
        outcome
            .instruction_list
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::Damage(damage) if damage.side_ref == side_ref => {
                    Some(damage.damage_amount)
                }
                _ => None,
            })
            .sum()
    }

    fn rolls(state: &State, move_id: Choices) -> Vec<i16> {
        calculate_damage_rolls(
            state.clone(),
            &SideReference::SideOne,
            MOVES.get(&move_id).unwrap().clone(),
            MOVES.get(&Choices::SPLASH).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn corrected_move_data_and_snipe_shot_power() {
        for (id, power, accuracy, move_type, category) in [
            (
                Choices::CRABHAMMER,
                100.0,
                95.0,
                PokemonType::WATER,
                MoveCategory::Physical,
            ),
            (
                Choices::GROWTH,
                0.0,
                100.0,
                PokemonType::GRASS,
                MoveCategory::Status,
            ),
            (
                Choices::MAKEITRAIN,
                120.0,
                95.0,
                PokemonType::STEEL,
                MoveCategory::Special,
            ),
            (
                Choices::METEORASSAULT,
                170.0,
                100.0,
                PokemonType::FIGHTING,
                MoveCategory::Physical,
            ),
            (
                Choices::SLASH,
                80.0,
                100.0,
                PokemonType::NORMAL,
                MoveCategory::Physical,
            ),
            (
                Choices::SWAGGER,
                0.0,
                85.0,
                PokemonType::NORMAL,
                MoveCategory::Status,
            ),
            (
                Choices::SYRUPBOMB,
                60.0,
                90.0,
                PokemonType::GRASS,
                MoveCategory::Special,
            ),
            (
                Choices::SNIPESHOT,
                85.0,
                100.0,
                PokemonType::WATER,
                MoveCategory::Special,
            ),
        ] {
            let actual = MOVES.get(&id).unwrap();
            assert_eq!(
                (
                    actual.base_power,
                    actual.accuracy,
                    actual.move_type,
                    actual.category
                ),
                (power, accuracy, move_type, category),
                "{:?}",
                id
            );
        }
    }

    #[test]
    fn corrected_moves_keep_their_accuracy_branches() {
        for (id, hit_chance) in [
            (Choices::CRABHAMMER, 95.0),
            (Choices::MAKEITRAIN, 95.0),
            (Choices::SWAGGER, 85.0),
            (Choices::SYRUPBOMB, 90.0),
        ] {
            let outcomes = use_move(&mut neutral_state(), id);
            assert_eq!(outcomes.len(), 2, "{:?}", id);
            let hit = outcomes
                .iter()
                .find(|x| !x.instruction_list.is_empty())
                .unwrap();
            let miss = outcomes
                .iter()
                .find(|x| x.instruction_list.is_empty())
                .unwrap();
            assert!((hit.percentage - hit_chance).abs() < 0.001, "{:?}", id);
            assert!(
                (miss.percentage - (100.0 - hit_chance)).abs() < 0.001,
                "{:?}",
                id
            );
        }
    }

    #[test]
    fn make_it_rain_keeps_its_two_stage_special_attack_drop() {
        let mut state = neutral_state();
        let outcomes = use_move(&mut state, Choices::MAKEITRAIN);
        let hit = outcomes
            .iter()
            .find(|x| damage_to(x, SideReference::SideTwo) > 0)
            .unwrap();
        assert_eq!(damage_to(hit, SideReference::SideTwo), 49);
        state.apply_instructions(&hit.instruction_list);
        assert_eq!(state.side_one.special_attack_boost, -2);
    }

    #[test]
    fn aura_guard_reduces_physical_special_and_fire_contact_attacks() {
        // These max/critical values use the existing effective-power model,
        // including its rounding, rather than halving already calculated damage.
        for (move_id, unguarded, guarded) in [
            (Choices::TACKLE, vec![19, 28], vec![10, 15]),
            (Choices::DRAININGKISS, vec![24, 36], vec![13, 19]),
            (Choices::FIREPUNCH, vec![35, 52], vec![18, 27]),
        ] {
            let mut state = neutral_state();
            assert_eq!(rolls(&state, move_id), unguarded, "{:?}", move_id);
            state.side_two.get_active().ability = Abilities::AURAGUARD;
            assert_eq!(rolls(&state, move_id), guarded, "{:?}", move_id);
        }
    }

    #[test]
    fn aura_guard_does_not_affect_noncontact_or_fixed_damage() {
        for (move_id, expected) in [
            (Choices::EARTHQUAKE, vec![46, 69]),
            (Choices::SHADOWBALL, vec![37, 55]),
            (Choices::SEISMICTOSS, vec![50]),
        ] {
            let mut state = neutral_state();
            state.side_two.get_active().ability = Abilities::AURAGUARD;
            assert_eq!(rolls(&state, move_id), expected, "{:?}", move_id);
        }
        let mut state = neutral_state();
        state.side_two.get_active().ability = Abilities::AURAGUARD;
        let outcomes = use_move(&mut state, Choices::SEISMICTOSS);
        assert_eq!(damage_to(&outcomes[0], SideReference::SideTwo), 50);
    }

    #[test]
    fn aura_guard_applies_during_instruction_generation() {
        let mut state = neutral_state();
        let unguarded = use_move(&mut state, Choices::TACKLE);
        assert_eq!(damage_to(&unguarded[0], SideReference::SideTwo), 17);
        state.side_two.get_active().ability = Abilities::AURAGUARD;
        let guarded = use_move(&mut state, Choices::TACKLE);
        assert_eq!(guarded.len(), 1);
        assert_eq!(damage_to(&guarded[0], SideReference::SideTwo), 9);
    }

    #[test]
    fn aura_guard_is_bypassed_or_suppressed_by_attacking_abilities() {
        for ability in [
            Abilities::MOLDBREAKER,
            Abilities::TERAVOLT,
            Abilities::TURBOBLAZE,
            Abilities::NEUTRALIZINGGAS,
            Abilities::LONGREACH,
        ] {
            let mut state = neutral_state();
            state.side_one.get_active().ability = ability;
            state.side_two.get_active().ability = Abilities::AURAGUARD;
            assert_eq!(
                rolls(&state, Choices::TACKLE),
                vec![19, 28],
                "{:?}",
                ability
            );
            let outcomes = use_move(&mut state, Choices::TACKLE);
            assert_eq!(
                damage_to(&outcomes[0], SideReference::SideTwo),
                17,
                "{:?}",
                ability
            );
        }
    }

    #[test]
    fn sunsteel_strike_bypasses_aura_guard() {
        let mut state = neutral_state();
        state.side_two.get_active().ability = Abilities::AURAGUARD;
        assert_eq!(rolls(&state, Choices::SUNSTEELSTRIKE), vec![46, 69]);
    }

    #[test]
    fn protective_pads_do_not_bypass_aura_guard() {
        let mut state = neutral_state();
        state.side_one.get_active().item = Items::PROTECTIVEPADS;
        state.side_two.get_active().ability = Abilities::AURAGUARD;
        assert_eq!(rolls(&state, Choices::TACKLE), vec![10, 15]);
        let outcomes = use_move(&mut state, Choices::TACKLE);
        assert_eq!(damage_to(&outcomes[0], SideReference::SideTwo), 9);
        assert_eq!(rolls(&state, Choices::SHADOWBALL), vec![37, 55]);

        state.side_one.get_active().ability = Abilities::LONGREACH;
        assert_eq!(rolls(&state, Choices::TACKLE), vec![19, 28]);
    }

    #[test]
    fn aura_guard_parses_and_round_trips_in_state() {
        assert_eq!(Abilities::from_str("auraguard"), Ok(Abilities::AURAGUARD));
        assert_eq!(Abilities::AURAGUARD.to_string(), "AURAGUARD");
        // New variants must not shift existing instruction ability identifiers.
        assert_eq!(Abilities::AURAGUARD as i16, Abilities::FIREMANE as i16 + 1);
        let mut state = neutral_state();
        state.side_two.get_active().ability = Abilities::AURAGUARD;
        state.side_two.get_active().base_ability = Abilities::AURAGUARD;
        let serialized = state.serialize();
        let restored = State::deserialize(&serialized);
        assert_eq!(
            restored.side_two.get_active_immutable().ability,
            Abilities::AURAGUARD
        );
        assert_eq!(
            restored.side_two.get_active_immutable().base_ability,
            Abilities::AURAGUARD
        );
        assert_eq!(restored.serialize(), serialized);
    }

    #[test]
    fn corrected_megas_evolve_with_expected_stats_types_and_abilities() {
        use PokemonType::*;
        for (base, item, mega, ability, base_types, types, base_stats) in [
            (
                PokemonName::ABSOL,
                Items::ABSOLITEZ,
                PokemonName::ABSOLMEGAZ,
                Abilities::SHARPNESS,
                (DARK, TYPELESS),
                (DARK, GHOST),
                (65, 154, 60, 75, 60, 151),
            ),
            (
                PokemonName::GARCHOMP,
                Items::GARCHOMPITEZ,
                PokemonName::GARCHOMPMEGAZ,
                Abilities::LEVITATE,
                (DRAGON, GROUND),
                (DRAGON, TYPELESS),
                (108, 130, 85, 141, 85, 151),
            ),
            (
                PokemonName::LUCARIO,
                Items::LUCARIONITEZ,
                PokemonName::LUCARIOMEGAZ,
                Abilities::AURAGUARD,
                (FIGHTING, STEEL),
                (FIGHTING, STEEL),
                (70, 100, 70, 164, 70, 151),
            ),
            (
                PokemonName::GOLISOPOD,
                Items::GOLISOPITE,
                PokemonName::GOLISOPODMEGA,
                Abilities::TOUGHCLAWS,
                (BUG, WATER),
                (BUG, STEEL),
                (75, 150, 175, 70, 120, 40),
            ),
            (
                PokemonName::STARMIE,
                Items::STARMINITE,
                PokemonName::STARMIEMEGA,
                Abilities::HUGEPOWER,
                (WATER, PSYCHIC),
                (WATER, PSYCHIC),
                (60, 100, 105, 130, 105, 120),
            ),
            (
                PokemonName::DRAMPA,
                Items::DRAMPANITE,
                PokemonName::DRAMPAMEGA,
                Abilities::BERSERK,
                (NORMAL, DRAGON),
                (NORMAL, DRAGON),
                (78, 85, 110, 160, 116, 36),
            ),
        ] {
            assert_eq!(mega.base_stats(), base_stats, "{:?}", mega);
            assert_eq!(
                base.mega_evolve_target(item).unwrap().base_stats,
                base_stats,
                "{:?}",
                mega
            );
            // At level 50 with a neutral nature and 31 IVs, 0 SP gives
            // base + 20; 11 SP gives base + 31 for each non-HP stat.
            for (sp, offset) in [(0, 20), (11, 31)] {
                let mut state = neutral_state();
                let pokemon = state.side_one.get_active();
                pokemon.id = base;
                pokemon.item = item;
                pokemon.types = base_types;
                pokemon.base_types = base_types;
                pokemon.evs = (sp, sp, sp, sp, sp, sp);
                let outcomes = generate(
                    &mut state,
                    MoveChoice::MoveMega(PokemonMoveIndex::M0),
                    MoveChoice::None,
                );
                assert_eq!(outcomes.len(), 1, "{:?}", mega);
                state.apply_instructions(&outcomes[0].instruction_list);
                let evolved = state.side_one.get_active_immutable();
                assert_eq!(evolved.id, mega);
                assert_eq!(evolved.ability, ability);
                assert_eq!(evolved.base_ability, ability);
                assert_eq!(evolved.types, types);
                assert!(evolved.mega_evolved);
                assert_eq!((evolved.hp, evolved.maxhp), (1000, 1000));
                assert_eq!(
                    (
                        evolved.attack,
                        evolved.defense,
                        evolved.special_attack,
                        evolved.special_defense,
                        evolved.speed
                    ),
                    (
                        base_stats.1 + offset,
                        base_stats.2 + offset,
                        base_stats.3 + offset,
                        base_stats.4 + offset,
                        base_stats.5 + offset
                    ),
                    "{:?} with {} SP",
                    mega,
                    sp
                );
                if mega == PokemonName::STARMIEMEGA && sp == 0 {
                    assert_eq!(evolved.attack, 120);
                    assert_eq!(rolls(&state, Choices::TACKLE), vec![44, 66]);
                }
            }
        }
    }

    #[test]
    fn mega_lucario_z_uses_aura_guard_before_a_faster_opponent_attacks() {
        let mut state = neutral_state();
        let lucario = state.side_one.get_active();
        lucario.id = PokemonName::LUCARIO;
        lucario.item = Items::LUCARIONITEZ;
        lucario.types = (PokemonType::FIGHTING, PokemonType::STEEL);
        lucario.base_types = lucario.types;
        state.side_two.get_active().speed = 500;
        state
            .side_two
            .get_active()
            .replace_move(PokemonMoveIndex::M0, Choices::TACKLE);
        let outcomes = generate(
            &mut state,
            MoveChoice::MoveMega(PokemonMoveIndex::M0),
            MoveChoice::Move(PokemonMoveIndex::M0),
        );
        assert_eq!(outcomes.len(), 1);
        assert_eq!(damage_to(&outcomes[0], SideReference::SideOne), 4);
        state.apply_instructions(&outcomes[0].instruction_list);
        assert_eq!(
            state.side_one.get_active_immutable().ability,
            Abilities::AURAGUARD
        );
    }
}

#[cfg(feature = "gen9")]
#[test]
fn champions_overrides_do_not_change_generation_nine_moves() {
    use poke_engine::choices::{Choices, MOVES};
    use poke_engine::state::PokemonType;

    for (move_id, power, accuracy) in [
        (Choices::CRABHAMMER, 100.0, 90.0),
        (Choices::MAKEITRAIN, 120.0, 100.0),
        (Choices::METEORASSAULT, 150.0, 100.0),
        (Choices::SLASH, 70.0, 100.0),
        (Choices::SWAGGER, 0.0, 90.0),
        (Choices::SYRUPBOMB, 60.0, 85.0),
        (Choices::SNIPESHOT, 80.0, 100.0),
    ] {
        let choice = MOVES.get(&move_id).unwrap();
        assert_eq!(
            (choice.base_power, choice.accuracy),
            (power, accuracy),
            "{:?}",
            move_id
        );
    }
    assert_eq!(
        MOVES.get(&Choices::GROWTH).unwrap().move_type,
        PokemonType::NORMAL
    );
    assert_eq!(
        MOVES
            .get(&Choices::MAKEITRAIN)
            .unwrap()
            .boost
            .as_ref()
            .unwrap()
            .boosts
            .special_attack,
        -1
    );
}
