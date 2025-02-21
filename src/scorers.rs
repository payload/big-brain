/*!
Scorers look at the world and boil down arbitrary characteristics into a range of 0.0..=1.0. This module includes the ScorerBuilder trait and some built-in Composite Scorers.
*/

use std::{cmp::Ordering, sync::Arc};

use bevy::prelude::*;

use crate::{
    evaluators::Evaluator,
    thinker::{Actor, ScorerEnt},
};

/**
Score value between `0.0..=1.0` associated with a Scorer.
 */
#[derive(Debug, Clone, Default)]
pub struct Score(pub(crate) f32);

impl Score {
    /**
    Returns the `Score`'s current value.
     */
    pub fn get(&self) -> f32 {
        self.0
    }
    /**
    Set the `Score`'s value.

    ### Panics

    Panics if `value` isn't within `0.0..=1.0`.
     */
    pub fn set(&mut self, value: f32) {
        if !(0.0..=1.0).contains(&value) {
            panic!("Score value must be between 0.0 and 1.0");
        }
        self.0 = value;
    }
}

/**
Trait that must be defined by types in order to be `ScorerBuilder`s. `ScorerBuilder`s' job is to spawn new `Scorer` entities. In general, most of this is already done for you, and the only method you really have to implement is `.build()`.

The `build()` method MUST be implemented for any `ScorerBuilder`s you want to define.
*/
pub trait ScorerBuilder: Sync + Send {
    /**
    MUST insert your concrete Scorer component into the Scorer [`Entity`], using `cmd`. You _may_ use `actor`, but it's perfectly normal to just ignore it.

    ### Example

    ```no_run
    struct MyBuilder;
    struct MyScorer;

    impl ScorerBuilder for MyBuilder {
        fn build(&self, cmd: &mut Commands, action: Entity, actor: Entity) {
            cmd.entity(action).insert(MyScorer);
        }
    }
    ```
    */
    fn build(&self, cmd: &mut Commands, scorer: Entity, actor: Entity);

    /**
    Don't implement this yourself unless you know what you're doing.
     */
    fn attach(&self, cmd: &mut Commands, actor: Entity) -> Entity {
        let scorer_ent = cmd.spawn().id();
        cmd.entity(scorer_ent)
            .insert(Name::new("Scorer"))
            .insert(Score::default())
            .insert(Actor(actor));
        self.build(cmd, scorer_ent, actor);
        scorer_ent
    }
}

/**
Scorer that always returns the same, fixed score. Good for combining with things creatively!
 */
#[derive(Debug, Clone)]
pub struct FixedScore(f32);

impl FixedScore {
    pub fn build(score: f32) -> FixedScoreBuilder {
        FixedScoreBuilder(score)
    }
}

pub fn fixed_score_system(mut query: Query<(&FixedScore, &mut Score)>) {
    for (FixedScore(fixed), mut score) in query.iter_mut() {
        score.set(*fixed);
    }
}

#[derive(Debug, Clone)]
pub struct FixedScoreBuilder(f32);

impl ScorerBuilder for FixedScoreBuilder {
    fn build(&self, cmd: &mut Commands, action: Entity, _actor: Entity) {
        cmd.entity(action).insert(FixedScore(self.0));
    }
}

/**
Composite Scorer that takes any number of other Scorers and returns the sum of their [`Score`] values if each _individual_ [`Score`] is at or above the configured `threshold`.

### Example

```ignore
Thinker::build()
    .when(
        AllOrNothing::build()
          .push(MyScorer)
          .push(MyOtherScorer),
        MyAction::build());
```
 */
#[derive(Debug)]
pub struct AllOrNothing {
    threshold: f32,
    scorers: Vec<ScorerEnt>,
}

impl AllOrNothing {
    pub fn build(threshold: f32) -> AllOrNothingBuilder {
        AllOrNothingBuilder {
            threshold,
            scorers: Vec::new(),
        }
    }
}

pub fn all_or_nothing_system(query: Query<(Entity, &AllOrNothing)>, mut scores: Query<&mut Score>) {
    for (
        aon_ent,
        AllOrNothing {
            threshold,
            scorers: children,
        },
    ) in query.iter()
    {
        let mut sum = 0.0;
        for ScorerEnt(child) in children.iter() {
            let score = scores.get_mut(*child).expect("where is it?");
            if score.0 < *threshold {
                sum = 0.0;
                break;
            } else {
                sum += score.0;
            }
        }
        let mut score = scores.get_mut(aon_ent).expect("where did it go?");
        score.set(crate::evaluators::clamp(sum, 0.0, 1.0));
    }
}
#[derive(Clone)]
pub struct AllOrNothingBuilder {
    threshold: f32,
    scorers: Vec<Arc<dyn ScorerBuilder>>,
}

impl AllOrNothingBuilder {
    /**
    Add another Scorer to this [`ScorerBuilder`].
     */
    pub fn push(mut self, scorer: impl ScorerBuilder + 'static) -> Self {
        self.scorers.push(Arc::new(scorer));
        self
    }
}

impl ScorerBuilder for AllOrNothingBuilder {
    fn build(&self, cmd: &mut Commands, scorer: Entity, actor: Entity) {
        let scorers: Vec<_> = self
            .scorers
            .iter()
            .map(|scorer| scorer.attach(cmd, actor))
            .collect();
        cmd.entity(scorer)
            .insert(Score::default())
            .push_children(&scorers[..])
            .insert(Name::new("Scorer"))
            .insert(AllOrNothing {
                threshold: self.threshold,
                scorers: scorers.into_iter().map(ScorerEnt).collect(),
            });
    }
}

/**
Composite Scorer that takes any number of other Scorers and returns the sum of their [`Score`] values if the _total_ summed [`Score`] is at or above the configured `threshold`.

### Example

```ignore
Thinker::build()
    .when(
        SumOfScorers::build()
          .push(MyScorer)
          .push(MyOtherScorer),
        MyAction::build());
```
 */
#[derive(Debug)]
pub struct SumOfScorers {
    threshold: f32,
    scorers: Vec<ScorerEnt>,
}

impl SumOfScorers {
    pub fn build(threshold: f32) -> SumOfScorersBuilder {
        SumOfScorersBuilder {
            threshold,
            scorers: Vec::new(),
        }
    }
}

pub fn sum_of_scorers_system(query: Query<(Entity, &SumOfScorers)>, mut scores: Query<&mut Score>) {
    for (
        sos_ent,
        SumOfScorers {
            threshold,
            scorers: children,
        },
    ) in query.iter()
    {
        let mut sum = 0.0;
        for ScorerEnt(child) in children.iter() {
            let score = scores.get_mut(*child).expect("where is it?");
            sum += score.0;
        }
        if sum < *threshold {
            sum = 0.0;
        }
        let mut score = scores.get_mut(sos_ent).expect("where did it go?");
        score.set(crate::evaluators::clamp(sum, 0.0, 1.0));
    }
}

#[derive(Clone)]
pub struct SumOfScorersBuilder {
    threshold: f32,
    scorers: Vec<Arc<dyn ScorerBuilder>>,
}

impl SumOfScorersBuilder {
    pub fn push(mut self, scorer: impl ScorerBuilder + 'static) -> Self {
        self.scorers.push(Arc::new(scorer));
        self
    }
}

impl ScorerBuilder for SumOfScorersBuilder {
    #[allow(clippy::needless_collect)]
    fn build(&self, cmd: &mut Commands, scorer: Entity, actor: Entity) {
        let scorers: Vec<_> = self
            .scorers
            .iter()
            .map(|scorer| scorer.attach(cmd, actor))
            .collect();
        cmd.entity(scorer)
            .insert(Transform::default())
            .insert(GlobalTransform::default())
            .insert(SumOfScorers {
                threshold: self.threshold,
                scorers: scorers.into_iter().map(ScorerEnt).collect(),
            });
    }
}

/**
Composite Scorer that takes any number of other Scorers and returns the single highest value [`Score`] if  _any_ [`Score`]s are at or above the configured `threshold`.

### Example

```ignore
Thinker::build()
    .when(
        WinningScorer::build()
          .push(MyScorer)
          .push(MyOtherScorer),
        MyAction::build());
```
 */

#[derive(Debug)]
pub struct WinningScorer {
    threshold: f32,
    scorers: Vec<ScorerEnt>,
}

impl WinningScorer {
    pub fn build(threshold: f32) -> WinningScorerBuilder {
        WinningScorerBuilder {
            threshold,
            scorers: Vec::new(),
        }
    }
}

pub fn winning_scorer_system(
    mut query: Query<(Entity, &mut WinningScorer)>,
    mut scores: QuerySet<(QueryState<&Score>, QueryState<&mut Score>)>,
) {
    for (sos_ent, mut winning_scorer) in query.iter_mut() {
        let (threshold, children) = (winning_scorer.threshold, &mut winning_scorer.scorers);
        let mut all_scores = children
            .iter()
            .map(|ScorerEnt(e)| scores.q0().get(*e).expect("where is it?").clone())
            .collect::<Vec<Score>>();

        all_scores.sort_by(|a, b| a.get().partial_cmp(&b.get()).unwrap_or(Ordering::Equal));
        let winning_score_or_zero = match all_scores.last() {
            Some(s) => {
                if s.get() < threshold {
                    0.0
                } else {
                    s.get()
                }
            }
            None => 0.0,
        };
        let mut q1 = scores.q1();
        let mut score = q1.get_mut(sos_ent).expect("where did it go?");
        score.set(crate::evaluators::clamp(winning_score_or_zero, 0.0, 1.0));
    }
}

#[derive(Clone)]
pub struct WinningScorerBuilder {
    threshold: f32,
    scorers: Vec<Arc<dyn ScorerBuilder>>,
}

impl WinningScorerBuilder {
    /**
    Add another Scorer to this [`ScorerBuilder`].
     */
    pub fn push(mut self, scorer: impl ScorerBuilder + 'static) -> Self {
        self.scorers.push(Arc::new(scorer));
        self
    }
}

impl ScorerBuilder for WinningScorerBuilder {
    #[allow(clippy::needless_collect)]
    fn build(&self, cmd: &mut Commands, scorer: Entity, actor: Entity) {
        let scorers: Vec<_> = self
            .scorers
            .iter()
            .map(|scorer| scorer.attach(cmd, actor))
            .collect();
        cmd.entity(scorer)
            .insert(Transform::default())
            .insert(GlobalTransform::default())
            .insert(WinningScorer {
                threshold: self.threshold,
                scorers: scorers.into_iter().map(ScorerEnt).collect(),
            });
    }
}

/**
Composite scorer that takes a `ScorerBuilder` and applies an `Evaluator`. Note that
unlike other composite scorers, `EvaluatingScorer` only takes one scorer upon building.

### Example

```ignore
Thinker::build()
    .when(
        EvaluatingScorer::build(MyScorer, MyEvaluator),
        MyAction::build());
```
 */
#[derive(Debug, Clone)]
pub struct EvaluatingScorer {
    scorer: ScorerEnt,
    evaluator: Arc<dyn Evaluator>,
}

impl EvaluatingScorer {
    pub fn build(
        scorer: impl ScorerBuilder + 'static,
        evaluator: impl Evaluator + 'static,
    ) -> EvaluatingScorerBuilder {
        EvaluatingScorerBuilder {
            evaluator: Arc::new(evaluator),
            scorer: Arc::new(scorer),
        }
    }
}

pub fn evaluating_scorer_system(
    query: Query<(Entity, &EvaluatingScorer)>,
    mut scores: QuerySet<(QueryState<&Score>, QueryState<&mut Score>)>,
) {
    for (sos_ent, eval_scorer) in query.iter() {
        // Get the inner score
        let inner_score = scores
            .q0()
            .get(eval_scorer.scorer.0)
            .expect("where did it go?")
            .get();
        // Get composite score
        let mut q1 = scores.q1();
        let mut score = q1.get_mut(sos_ent).expect("where did it go?");
        score.set(crate::evaluators::clamp(
            eval_scorer.evaluator.evaluate(inner_score),
            0.0,
            1.0,
        ));
    }
}

#[derive(Clone)]
pub struct EvaluatingScorerBuilder {
    pub scorer: Arc<dyn ScorerBuilder>,
    pub evaluator: Arc<dyn Evaluator>,
}

impl ScorerBuilder for EvaluatingScorerBuilder {
    fn build(&self, cmd: &mut Commands, scorer: Entity, actor: Entity) {
        let inner_scorer = self.scorer.attach(cmd, actor);
        cmd.entity(scorer)
            .insert(Transform::default())
            .insert(GlobalTransform::default())
            .insert(EvaluatingScorer {
                evaluator: self.evaluator.clone(),
                scorer: ScorerEnt(inner_scorer),
            });
    }
}
