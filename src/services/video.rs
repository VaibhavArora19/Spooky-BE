use crate::{RoomSync, ws_conn::SyncInfo};

pub async fn set_sync_info(room_id: String, sync_info: SyncInfo, room_sync: RoomSync) {
    let mut room_sync_write = room_sync.write().await;

    let sync_info = SyncInfo {
        last_action: sync_info.last_action.clone(),
        time: sync_info.time,
        updated_at: sync_info.updated_at,
        updated_by: sync_info.updated_by.clone(),
    };

    room_sync_write.insert(room_id.clone(), sync_info);
}

pub async fn get_sync_info(room_id: String, room_sync: RoomSync) -> Option<SyncInfo> {
    let room_sync_write = room_sync.read().await;

    room_sync_write.get(&room_id).cloned()
}
