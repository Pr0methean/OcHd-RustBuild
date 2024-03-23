use crate::image_tasks::color::c;
use crate::image_tasks::task_spec::{
    from_svg_task, out_task, paint_svg_task, FileOutputTaskSpec, ToPixmapTaskSpec,
};
use crate::texture_base::material::{ColorTriad, Material};
use crate::{stack, stack_on};

struct CommandBlockSideType {
    name: &'static str,
    layer_under_grid_name: &'static str,
    grid_layer_name: &'static str,
}

impl CommandBlockSideType {
    fn grid_layers(&self, type_colors: &ColorTriad) -> ToPixmapTaskSpec {
        stack!(
            paint_svg_task(self.layer_under_grid_name, type_colors.shadow),
            from_svg_task(self.grid_layer_name)
        )
    }
}

pub struct CommandBlockColorType {
    prefix: &'static str,
    colors: ColorTriad,
    decoration: Option<&'static str>,
}

const FRONT: CommandBlockSideType = CommandBlockSideType {
    name: "front",
    layer_under_grid_name: "commandBlockOctagon4x",
    grid_layer_name: "commandBlockGridFront",
};

const BACK: CommandBlockSideType = CommandBlockSideType {
    name: "back",
    layer_under_grid_name: "commandBlockSquare4x",
    grid_layer_name: "commandBlockGrid",
};

const SIDE: CommandBlockSideType = CommandBlockSideType {
    name: "side",
    layer_under_grid_name: "commandBlockArrowUnconditional4x",
    grid_layer_name: "commandBlockGrid",
};

const CONDITIONAL: CommandBlockSideType = CommandBlockSideType {
    name: "conditional",
    layer_under_grid_name: "commandBlockArrow4x",
    grid_layer_name: "commandBlockGrid",
};

const SIDE_TYPES: &[CommandBlockSideType] = &[FRONT, BACK, SIDE, CONDITIONAL];

pub const COMMAND_BLOCK: CommandBlockColorType = CommandBlockColorType {
    prefix: "",
    colors: ColorTriad {
        color: c(0xc77e4f),
        shadow: c(0xa66030),
        highlight: c(0xd7b49d),
    },
    decoration: None,
};

pub const CHAIN_COMMAND_BLOCK: CommandBlockColorType = CommandBlockColorType {
    prefix: "chain_",
    colors: ColorTriad {
        color: c(0x76b297),
        shadow: c(0x5f8f7a),
        highlight: c(0xA8BEC5),
    },
    decoration: Some("commandBlockChains4x"),
};

pub const REPEATING_COMMAND_BLOCK: CommandBlockColorType = CommandBlockColorType {
    prefix: "repeating_",
    colors: ColorTriad {
        color: c(0x6a4fc7),
        shadow: c(0x553b9b),
        highlight: c(0x9b8bcf),
    },
    decoration: Some("loopArrow4x"),
};

const COLOR_TYPES: &[CommandBlockColorType] =
    &[COMMAND_BLOCK, CHAIN_COMMAND_BLOCK, REPEATING_COMMAND_BLOCK];

pub enum CommandBlocks {
    CommandBlocks,
}

impl Material for CommandBlocks {
    fn get_output_tasks(&self) -> Box<[FileOutputTaskSpec]> {
        COLOR_TYPES
            .iter()
            .flat_map(|color_type| {
                let background = stack_on!(
                    color_type.colors.color,
                    paint_svg_task("diagonalChecks4x", color_type.colors.shadow),
                    paint_svg_task("diagonalChecksFill4x", color_type.colors.highlight)
                );
                let decorated_background = if let Some(decoration) = &color_type.decoration {
                    stack!(background, from_svg_task(*decoration))
                } else {
                    background
                };
                SIDE_TYPES.iter().map(move |side_type| {
                    out_task(
                        format!(
                            "block/{}command_block_{}",
                            color_type.prefix, side_type.name
                        ),
                        stack!(
                            decorated_background.to_owned(),
                            side_type.grid_layers(&color_type.colors)
                        ),
                    )
                })
            })
            .collect()
    }
}
