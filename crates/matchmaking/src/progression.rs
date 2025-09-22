use uuid::Uuid;

/// Gets player progression
pub struct Progression {
    pub level: u32,
    pub xp: u32,
    pub loadouts_id: Vec<u8>,
    pub skills_unlocked: Vec<Uuid>,
    pub inventory_items: Vec<InventoryItems>,
}

/// Inventory items
pub struct InventoryItems {
    pub id: Uuid,
    pub rolls: Vec<Uuid>,
    pub rarity: u8,
}
