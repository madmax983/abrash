use proptest::prelude::*;
use crate::sequence::Sequence;
use crate::evaluable::{Evaluable, Sample};
use crate::keyframe::Keyframe;
use crate::easing::Easing;

proptest! {
    #[test]
    fn test_sequence_fuzz(phases in prop::collection::vec(0.0f32..1.0f32, 1..100)) {
        let mut segments: Vec<Box<dyn Evaluable<f32>>> = vec![];
        for _ in 0..10 {
            segments.push(Box::new(Keyframe::new(0.0_f32, 10.0, Easing::Linear, 1.0)));
        }
        let seq = Sequence::new(segments);
        for phase in phases {
            let _ = Evaluable::evaluate(&seq, phase);
        }
    }
}
