#![forbid(unsafe_code)]

//! Modal container widget (overlay layer).

mod container;

pub use container::{
    BackdropConfig, MODAL_HIT_BACKDROP, MODAL_HIT_CONTENT, Modal, ModalAction, ModalConfig,
    ModalPosition, ModalSizeConstraints, ModalState,
};
