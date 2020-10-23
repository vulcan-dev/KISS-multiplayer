pub mod electrics;
pub mod gearbox;
pub mod transform;

pub use electrics::*;
pub use gearbox::*;
pub use transform::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VehicleData {
    pub parts_config: String,
    pub in_game_id: u32,
    pub color: [f32; 4],
    pub palete_0: [f32; 4],
    pub palete_1: [f32; 4],
    pub name: String,
    pub server_id: Option<u32>,
    pub owner: Option<u32>,
}

#[derive(Clone)]
pub struct Vehicle {
    pub transform: Option<Transform>,
    pub electrics: Option<Electrics>,
    pub gearbox: Option<Gearbox>,
    pub data: VehicleData,
}

impl crate::Server {
    pub async fn remove_vehicle(&mut self, id: u32, client_id: Option<u32>) {
        self.vehicles.remove(&id);
        for (_, client) in &mut self.connections {
            client
                .unordered
                .send(crate::Outgoing::RemoveVehicle(id))
                .await
                .unwrap();
        }
        if let Some(client_id) = client_id {
            self.vehicle_ids.remove(&client_id);
        }
    }
    pub async fn reset_vehicle(&mut self, server_id: u32, client_id: Option<u32>) {
        for (cid, client) in &mut self.connections {
            if client_id.is_some() && *cid == client_id.unwrap() {
                continue;
            }
            client
                .unordered
                .send(crate::Outgoing::ResetVehicle(server_id))
                .await
                .unwrap();
        }
    }
    pub fn get_server_id_from_game_id(&self, client_id: u32, game_id: u32) -> Option<u32> {
        if let Some(client_vehicles) = self.vehicle_ids.get(&client_id) {
            if let Some(server_id) = client_vehicles.get(&game_id) {
                Some(*server_id)
            } else {
                None
            }
        } else {
            None
        }
    }
}
