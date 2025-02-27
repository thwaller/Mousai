use futures_channel::oneshot;
use gtk::glib;

use std::{cell::Cell, rc::Rc, time::Duration};

use crate::core::{Cancellable, Cancelled};

/// Send something to a [`glib::Sender`]
#[macro_export]
macro_rules! send {
    ($sender:expr, $action:expr) => {
        if let Err(err) = $sender.send($action) {
            log::error!("Failed to send \"{}\" action: {err:?}", stringify!($action));
        }
    };
}

/// Spawns a future in the default [`glib::MainContext`]
pub fn spawn<F: std::future::Future<Output = ()> + 'static>(fut: F) {
    let ctx = glib::MainContext::default();
    ctx.spawn_local(fut);
}

/// Like [`glib::timeout_future`] but terminates immediately after `cancellable`
/// is cancelled and return an error.
pub async fn timeout_future(
    interval: Duration,
    cancellable: &Cancellable,
) -> Result<(), Cancelled> {
    let (sender, receiver) = oneshot::channel();
    let is_source_removed = Rc::new(Cell::new(false));
    let is_source_removed_clone = Rc::clone(&is_source_removed);

    let source_id = glib::timeout_add_local_once(interval, move || {
        let _ = sender.send(());
        is_source_removed.set(true);
    });

    cancellable.connect_cancelled(move |_| {
        if !is_source_removed_clone.get() {
            // Once source is removed the sender that was moved in the closure
            // will be dropped. Causing the receiver to emit `oneshot::Canceled`.
            source_id.remove();
        }
    });

    receiver.await.map_err(|_| Cancelled::default())
}
