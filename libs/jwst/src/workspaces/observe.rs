use nanoid::nanoid;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    thread::sleep,
    time::Duration,
};
use y_sync::awareness::{Awareness, Event, Subscription as AwarenessSubscription};
use yrs::{types::map::MapEvent, Observable, TransactionMut, UpdateEvent};

use super::*;

impl Workspace {
    pub fn observe_metadata(
        &mut self,
        f: impl Fn(&TransactionMut, &MapEvent) + 'static,
    ) -> MapSubscription {
        self.metadata.observe(f)
    }

    pub async fn on_awareness_update(
        &mut self,
        f: impl Fn(&Awareness, &Event) + 'static,
    ) -> AwarenessSubscription<Event> {
        self.awareness.write().await.on_update(f)
    }

    /// Subscribe to update events.
    pub async fn observe(
        &mut self,
        f: impl Fn(&TransactionMut, &UpdateEvent) + Clone + 'static,
    ) -> Option<String> {
        info!("workspace observe enter");
        let doc = self.doc();
        match catch_unwind(AssertUnwindSafe(move || {
            let mut retry = 10;
            let cb = move |trx: &TransactionMut, evt: &UpdateEvent| {
                trace!("workspace observe: observe_update_v1, {:?}", &evt.update);
                if let Err(e) = catch_unwind(AssertUnwindSafe(|| f(trx, evt))) {
                    error!("panic in observe callback: {:?}", e);
                }
            };

            loop {
                match doc.observe_update_v1(cb.clone()) {
                    Ok(sub) => break Ok(sub),
                    Err(e) if retry <= 0 => break Err(e),
                    _ => {
                        sleep(Duration::from_micros(100));
                        retry -= 1;
                        continue;
                    }
                }
            }
        })) {
            Ok(sub) => match sub {
                Ok(sub) => {
                    let id = nanoid!();
                    self.sub.write().await.insert(id.clone(), sub);
                    Some(id)
                }
                Err(e) => {
                    error!("failed to observe: {:?}", e);
                    None
                }
            },
            Err(e) => {
                error!("panic in observe callback: {:?}", e);
                None
            }
        }
    }
}