//! In-process per–API key concurrency limiting (one gateway instance; not distributed).

use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub type KeyConcurrencyRegistry = Arc<DashMap<i64, (u32, Arc<Semaphore>)>>;

/// Acquire a slot for one upstream chat request. Returns [`None`] when no limit is configured.
/// Hold the permit until the response body is fully consumed (including SSE streams).
pub async fn acquire_chat_slot(
    registry: &KeyConcurrencyRegistry,
    key_id: i64,
    max_concurrent: Option<i32>,
) -> Option<OwnedSemaphorePermit> {
    let cap = match max_concurrent {
        None => {
            registry.remove(&key_id);
            return None;
        }
        Some(m) if m <= 0 => {
            registry.remove(&key_id);
            return None;
        }
        Some(m) => (m as u32).clamp(1, 65_535),
    };
    let sem = match registry.entry(key_id) {
        Entry::Occupied(mut o) => {
            if o.get().0 != cap {
                o.insert((cap, Arc::new(Semaphore::new(cap as usize))));
            }
            o.get().1.clone()
        }
        Entry::Vacant(v) => {
            let arc = Arc::new(Semaphore::new(cap as usize));
            v.insert((cap, arc.clone()));
            arc
        }
    };
    Some(sem.acquire_owned().await.expect("semaphore not closed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn unlimited_returns_none_and_removes_registry_entry() {
        let reg: KeyConcurrencyRegistry = Arc::new(DashMap::new());
        let key = 1_i64;
        let _p = acquire_chat_slot(&reg, key, Some(1))
            .await
            .expect("first slot");
        assert!(reg.contains_key(&key));

        assert!(acquire_chat_slot(&reg, key, None).await.is_none());
        assert!(!reg.contains_key(&key));
    }

    #[tokio::test]
    async fn zero_or_negative_max_returns_none_and_clears() {
        let reg: KeyConcurrencyRegistry = Arc::new(DashMap::new());
        let key = 2_i64;
        let _p = acquire_chat_slot(&reg, key, Some(2)).await.expect("slot");
        assert!(reg.contains_key(&key));

        assert!(acquire_chat_slot(&reg, key, Some(0)).await.is_none());
        assert!(!reg.contains_key(&key));

        let _p2 = acquire_chat_slot(&reg, key, Some(2))
            .await
            .expect("recreated");
        assert!(acquire_chat_slot(&reg, key, Some(-1)).await.is_none());
        assert!(!reg.contains_key(&key));
    }

    #[tokio::test]
    async fn cap_two_allows_two_immediate_acquires() {
        let reg: KeyConcurrencyRegistry = Arc::new(DashMap::new());
        let key = 3_i64;
        let _a = acquire_chat_slot(&reg, key, Some(2)).await.expect("a");
        let _b = acquire_chat_slot(&reg, key, Some(2)).await.expect("b");
    }

    #[tokio::test]
    async fn third_acquire_blocks_until_a_slot_is_released() {
        let reg: KeyConcurrencyRegistry = Arc::new(DashMap::new());
        let key = 4_i64;
        let p1 = acquire_chat_slot(&reg, key, Some(2)).await.expect("p1");
        let p2 = acquire_chat_slot(&reg, key, Some(2)).await.expect("p2");

        let (tx, mut rx) = mpsc::channel::<()>(1);
        let reg2 = reg.clone();
        tokio::spawn(async move {
            let _guard = acquire_chat_slot(&reg2, key, Some(2))
                .await
                .expect("third acquire");
            let _ = tx.try_send(());
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(matches!(
            rx.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));

        drop(p1);

        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("third acquire should complete after a slot is freed")
            .expect("sender dropped");

        drop(p2);
    }

    #[tokio::test]
    async fn cap_change_rebuilds_semaphore_so_new_acquires_use_new_limit() {
        let reg: KeyConcurrencyRegistry = Arc::new(DashMap::new());
        let key = 5_i64;
        let _a = acquire_chat_slot(&reg, key, Some(1)).await.expect("a");
        // New limit: second acquire on updated cap uses a fresh semaphore (2 permits).
        let _b = acquire_chat_slot(&reg, key, Some(2))
            .await
            .expect("b after cap raised");
        let _c = acquire_chat_slot(&reg, key, Some(2)).await.expect("c");
    }
}
