use lazy_static::lazy_static;
use crate::image_tasks::color::c;
use crate::image_tasks::task_spec::{FileOutputTaskSpec, from_svg_task, out_task, paint_svg_task, ToPixmapTaskSpec};
use crate::{group, stack, stack_on};
use crate::texture_base::material::{ColorTriad, Material};

struct CommandBlockSideType {
    name: &'static str,
    grid_layers: Box<dyn Fn(&ColorTriad) -> ToPixmapTaskSpec + Send + Sync>
}

struct CommandBlockColorType {
    prefix: &'static str,
    colors: ColorTriad,
    decoration: Option<ToPixmapTaskSpec>
}

lazy_static! {
    static ref FRONT: CommandBlockSideType = CommandBlockSideType {
        name: "front",
        grid_layers: Box::new(|colors| stack!(
            paint_svg_task("commandBlockOctagon4x", colors.shadow),
            from_svg_task("commandBlockGridFront")
        ))
    };

    static ref BACK: CommandBlockSideType = CommandBlockSideType {
        name: "back",
        grid_layers: Box::new(|colors| stack!(
            paint_svg_task("commandBlockSquare4x", colors.shadow),
            from_svg_task("commandBlockGrid")
        ))
    };

    static ref SIDE: CommandBlockSideType = CommandBlockSideType {
        name: "side",
        grid_layers: Box::new(|colors| stack!(
            paint_svg_task("commandBlockArrowUnconditional4x", colors.shadow),
            from_svg_task("commandBlockGrid")
        ))
    };

    static ref CONDITIONAL: CommandBlockSideType = CommandBlockSideType {
        name: "conditional",
        grid_layers: Box::new(|colors| stack!(
            paint_svg_task("commandBlockArrow4x", colors.shadow),
            from_svg_task("commandBlockGrid")
        ))
    };

    static ref SIDE_TYPES: Vec<&'static CommandBlockSideType>
        = vec![&FRONT, &BACK, &SIDE, &CONDITIONAL];

    static ref COMMAND_BLOCK: CommandBlockColorType = CommandBlockColorType {
        prefix: "",
        colors: ColorTriad {color: c(0xc77e4f), shadow: c(0xa66030), highlight: c(0xd7b49d)},
        decoration: None
    };

    static ref CHAIN_COMMAND_BLOCK: CommandBlockColorType = CommandBlockColorType {
        prefix: "chain_",
        colors: ColorTriad {color: c(0x76b297), shadow: c(0x5f8f7a), highlight: c(0xA8BEC5)},
        decoration: Some(from_svg_task("commandBlockChains4x"))
    };

    static ref REPEATING_COMMAND_BLOCK: CommandBlockColorType = CommandBlockColorType {
        prefix: "repeating_",
        colors: ColorTriad {color: c(0x6a4fc7), shadow: c(0x553b9b), highlight: c(0x9b8bcf)},
        decoration: Some(from_svg_task("loopArrow4x"))
    };
}

impl Material for CommandBlockColorType {
    fn get_output_tasks(&self) -> Vec<FileOutputTaskSpec> {
        let background = stack_on!(
                    self.colors.color,
                    paint_svg_task("diagonalChecks4x", self.colors.shadow),
                    paint_svg_task("diagonalChecksFill4x", self.colors.highlight));
        let decorated_background = if let Some(decoration) = &self.decoration {
            stack!(background, decoration.to_owned())
        } else {
            background
        };
        SIDE_TYPES.iter().map(|side_type| {
            out_task(&format!("block/{}command_block_{}", self.prefix, side_type.name),
                stack!(
                    decorated_background.to_owned(),
                    (side_type.grid_layers)(&self.colors)
                )
            )
        }).collect()
    }
}

group!(COMMAND_BLOCKS = COMMAND_BLOCK, CHAIN_COMMAND_BLOCK, REPEATING_COMMAND_BLOCK);
