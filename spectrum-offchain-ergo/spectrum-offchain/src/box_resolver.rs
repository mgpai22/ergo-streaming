use std::sync::Arc;

use tokio::sync::Mutex;

use crate::box_resolver::persistence::EntityRepo;
use crate::data::unique_entity::{Confirmed, Predicted, Traced, Unconfirmed};
use crate::data::OnChainEntity;

pub mod persistence;
pub mod process;
pub mod rocksdb;
pub mod blacklist;

/// Get latest state of an on-chain entity `TEntity`.
pub async fn resolve_entity_state<TEntity, TRepo>(
    id: TEntity::TEntityId,
    repo: Arc<Mutex<TRepo>>,
) -> Option<TEntity>
where
    TRepo: EntityRepo<TEntity>,
    TEntity: OnChainEntity,
    TEntity::TEntityId: Copy,
{
    let states = {
        let repo_guard = repo.lock().await;
        let confirmed = repo_guard.get_last_confirmed(id).await;
        let unconfirmed = repo_guard.get_last_unconfirmed(id).await;
        let predicted = repo_guard.get_last_predicted(id).await;
        (confirmed, unconfirmed, predicted)
    };
    match states {
        (Some(Confirmed(conf)), unconf, Some(Predicted(pred))) => {
            let anchoring_point = unconf.map(|Unconfirmed(e)| e).unwrap_or(conf);
            let anchoring_sid = anchoring_point.get_self_state_ref();
            let predicted_sid = pred.get_self_state_ref();
            let prediction_is_anchoring_point = predicted_sid == anchoring_sid;
            let prediction_is_valid = prediction_is_anchoring_point
                || is_linking(predicted_sid, anchoring_sid, Arc::clone(&repo)).await;
            let safe_point = if prediction_is_valid {
                pred
            } else {
                anchoring_point
            };
            Some(safe_point)
        }
        (_, Some(Unconfirmed(unconf)), None) => Some(unconf),
        (Some(Confirmed(conf)), _, _) => Some(conf),
        _ => None,
    }
}

async fn is_linking<TEntity, TRepo>(
    sid: TEntity::TStateId,
    anchoring_sid: TEntity::TStateId,
    repo: Arc<Mutex<TRepo>>,
) -> bool
where
    TEntity: OnChainEntity,
    TRepo: EntityRepo<TEntity>,
{
    let mut head_sid = sid;
    let repo = repo.lock().await;
    loop {
        match repo.get_prediction_predecessor(head_sid).await {
            None => return false,
            Some(prev_state_id) if prev_state_id == anchoring_sid => return true,
            Some(prev_state_id) => head_sid = prev_state_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sigma_test_util::force_any_val;
    use tokio::sync::Mutex;

    use crate::box_resolver::persistence::tests::*;
    use crate::box_resolver::persistence::EntityRepo;
    use crate::box_resolver::resolve_entity_state;
    use crate::data::unique_entity::Confirmed;
    use crate::data::OnChainEntity;

    #[tokio::test]
    async fn test_resolve_state_trivial() {
        let mut client = rocks_db_client();
        let entity = Confirmed(ErgoEntity {
            token_id: force_any_val(),
            box_id: force_any_val(),
        });
        client.put_confirmed(entity.clone()).await;

        let client = Arc::new(Mutex::new(client));
        let resolved = resolve_entity_state::<ErgoEntity, _>(entity.0.get_self_ref(), client).await;
        assert_eq!(resolved, Some(entity.0));
    }
}
