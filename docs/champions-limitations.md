# Pokémon Champions limitations

The `champions` feature includes Champions move data, Mega Evolution data, and
Aura Guard. These fixes do not establish complete Pokémon Champions support.
The engine remains a singles battle engine with some approximated mechanics.

## Deferred move effects

The following moves are recognized by name and have move definitions, but their
battle effects are not implemented. All nine are deferred and are not implemented
as part of the Mega data, move data, and Aura Guard fixes.

| Move | Intended effect and missing behavior |
| --- | --- |
| Heal Pulse | Restore the target's HP; target healing is not implemented. |
| Magic Powder | Change an eligible target's type to Psychic; the type change is not implemented. |
| Metronome | Randomly call an eligible move; move selection and execution are not implemented. |
| Revival Blessing | Select and revive a fainted party member at half HP; selection and revival are not implemented. |
| Simple Beam | Change an eligible target's ability to Simple; the ability change is not implemented. |
| Soak | Change an eligible target's type to Water; the type change is not implemented. |
| Speed Swap | Exchange the user's and target's Speed stats; the swap is not implemented. |
| Transform | Copy the target's form, types, ability, stats, and moves under the move's rules; transformation is not implemented. |
| Worry Seed | Change an eligible target's ability to Insomnia; the ability change is not implemented. |

Their presence in the move registry must not be interpreted as full support for
their effects. Other interactions have not been exhaustively verified either.

## Aura Guard's damage model

Aura Guard uses the engine's existing defender-ability model: it multiplies a
damaging contact move's effective base power by 0.5. This approximates halving
damage, and rounding can differ from applying a multiplier to calculated damage.
There is no additional Fire weakness. Non-contact attacks and fixed-damage moves
are unaffected.

Long Reach prevents contact. Protective Pads do not bypass Aura Guard. Aura Guard
is suppressed by Neutralizing Gas and bypassed by Mold Breaker, Teravolt,
Turboblaze, and applicable ability-ignoring moves, following the audited Showdown
implementation.

## Reference data and remaining scope

The corrections use the [PokéBase Pokémon list](https://pokebase.app/pokemon-champions/pokemon),
[PokéBase move list](https://pokebase.app/pokemon-champions/moves), and the local
Pokémon Showdown Champions data audited on 2026-10-24. Snipe Shot has **85 base
power in Champions**, matching Showdown and the explicit requirement for this fix,
despite PokéBase listing 80 at the time of the audit.

Champions move overrides do not change other generations' move definitions.
Ditto's Imposter, doubles support, multi-hit approximations, and other audit
findings are outside this fix.
