# big-brain

`big-brain` is a [Utility AI](https://en.wikipedia.org/wiki/Utility_system)
library for games, built for the [Bevy Game Engine](https://bevyengine.org/)

It lets you define complex, intricate AI behaviors for your entities based on
their perception of the world. Definitions are heavily data-driven, using
plain Rust, and you only need to program Scorers (entities that look at your
game world and come up with a Score), and Actions (entities that perform
actual behaviors upon the world). No other code is needed for actual AI
behavior.

See [the documentation](https://docs.rs/big-brain) for more details.

### Features

* Highly concurrent/parallelizable evaluation.
* Integrates smoothly with Bevy.
* Easy AI definition using idiomatic Rust builders. You don't have to be some genius to define behavior that _feels_ realistic to players.
* High performance--supports hundreds of thousands of concurrent AIs.
* Graceful degradation--can be configured such that the less frame time is available, the slower an AI might "seem", without dragging down framerates, by simply processing fewer events per tick.
* Proven game AI model.
* Low code overhead--you only define two types of application-dependent things, and everything else is building blocks!
* Highly composable and reusable.
* State machine-style continuous actions/behaviors.
* Action cancellation.

### Example

First, you define actions and considerations, which are just plain old Bevy
Components and Systems. As a developer, you write application-dependent code
to define [`Scorers`](#scorers) and [`Actions`](#actions), and then put it
all together like building blocks, using [`Thinkers`](#thinkers) that will
define the actual behavior.

#### Scorers

`Scorer`s are entities that look at the world and evaluate into [`Score`](scorers::Score) values. You can think of them as the "eyes" of the AI system. They're a highly-parallel way of being able to look at the `World` and use it to make some decisions later.

They are created by types that implement [`ScorerBuilder`](scorers::ScorerBuilder).

```rust
use bevy::prelude::*;
use big_brain::prelude::*;

#[derive(Debug, Clone)]
pub struct Thirsty;

impl Thirsty {
    fn build() -> ThirstyBuilder {
        ThirstyBuilder
    }
}

#[derive(Debug, Clone)]
pub struct ThirstyBuilder;

impl ScorerBuilder for ThirstyBuilder {
    fn build(&self, cmd: &mut Commands, scorer: Entity, _actor: Entity) {
        cmd.entity(scorer).insert(Thirsty);
    }
}

pub fn thirsty_scorer_system(
    thirsts: Query<&Thirst>,
    mut query: Query<(&Actor, &mut Score), With<Thirsty>>,
) {
    for (Actor(actor), mut score) in query.iter_mut() {
        if let Ok(thirst) = thirsts.get(*actor) {
            score.set(thirst.thirst);
        }
    }
}
```

#### Actions

`Action`s are the actual things your entities will _do_. They are connected to [`ActionState`](actions::ActionState)s, and are created by types implementing [`ActionBuilder`](actions::ActionBuilder).

```rust
use bevy::prelude::*;
use big_brain::prelude::*;

#[derive(Debug, Clone)]
pub struct Drink;

impl Drink {
    pub fn build() -> DrinkBuilder {
        DrinkBuilder
    }
}

#[derive(Debug, Clone)]
pub struct DrinkBuilder;

impl ActionBuilder for DrinkBuilder {
    fn build(&self, cmd: &mut Commands, action: Entity, _actor: Entity) {
        cmd.entity(action).insert(Drink);
    }
}

fn drink_action_system(
    mut thirsts: Query<&mut Thirst>,
    mut query: Query<(&Actor, &mut ActionState), With<Drink>>,
) {
    for (Actor(actor), mut state) in query.iter_mut() {
        if let Ok(mut thirst) = thirsts.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    thirst.thirst = 10.0;
                    *state = ActionState::Success;
                }
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}
```

#### Thinkers

Finally, you can use it when define the [`Thinker`](thinker::Thinker), which you can attach as a
regular Component:

```rust
cmd.spawn().insert(Thirst::new(70.0, 2.0)).insert(
    Thinker::build()
        .picker(FirstToScore { threshold: 0.8 })
        .when(Thirsty::build(), Drink::build()),
);
```

### Contributing

1. Install the latest Rust toolchain (stable supported).
2. `cargo run --example thirst`
3. Happy hacking!

### License

This project is licensed under [the Apache-2.0 License](LICENSE.md).
