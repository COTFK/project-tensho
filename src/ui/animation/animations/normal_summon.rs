use super::super::{Animation, AnimationBounds};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormalSummon;

impl Animation for NormalSummon {
    fn keyframes(&self, source: AnimationBounds, destination: AnimationBounds) -> String {
        let destination_scale = if source.width > 0.0 {
            destination.width / source.width
        } else {
            1.0
        };
        let lift_scale = 1.5;
        let lift_center_x = source.x
            + source.width / 2.0
            + ((destination.x + destination.width / 2.0) - (source.x + source.width / 2.0)) * 0.8;
        let lift_center_y = source.y
            + source.height / 2.0
            + ((destination.y + destination.height / 2.0) - (source.y + source.height / 2.0)) * 0.8;
        let lift_x = lift_center_x - source.width * lift_scale / 2.0;
        let lift_y = lift_center_y - source.height * lift_scale / 2.0;

        format!(
            "@keyframes normal-summon {{
            0% {{
                transform: translate3d({}px, {}px, 0) scale(1.0);
                animation-timing-function: cubic-bezier(0.2, 0.8, 0.2, 1);
            }}
            75% {{
                transform: translate3d({lift_x}px, {lift_y}px, 0) scale({lift_scale});
                animation-timing-function: cubic-bezier(0.55, 0, 0.9, 0.35);
            }}
            100% {{ transform: translate3d({}px, {}px, 0) scale({destination_scale}); }}
        }}",
            source.x, source.y, destination.x, destination.y,
        )
    }

    fn parameters(&self) -> &'static str {
        "transform-origin: top left; animation: normal-summon 450ms linear forwards;"
    }
}
