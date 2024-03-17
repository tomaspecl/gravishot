// Gravishot
// Copyright (C) 2024 Tomáš Pecl
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::EntityType;
use crate::bullet::SpawnBullet;
use crate::player::SpawnPlayer;
use crate::input::Inputs;

use bevy_gravirollback::new::for_user::*;
use bevy_gravirollback::new::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy::ecs::system::StaticSystemParam;
use bevy::utils::HashMap;
use bevy::ecs::query::WorldQuery;
use serde::{Serialize, Deserialize};

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub static ROLLBACK_ID_COUNTER: RollbackIdCounter = RollbackIdCounter(AtomicU64::new(0));
pub struct RollbackIdCounter(pub AtomicU64);
impl RollbackIdCounter {
    pub fn get_new(&self) -> RollbackID {
        RollbackID(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Bundle, Reflect, Serialize, Deserialize, Default, Clone, Debug)]
pub struct PhysicsBundle {
    pub transform: Transform,
    pub velocity: Velocity,
}

impl RollbackCapable for PhysicsBundle {
    type RestoreQuery<'a> = (&'a mut Transform, &'a mut Velocity);
    type RestoreExtraParam = ();
    type SaveQuery<'a> = (&'a Transform, &'a Velocity);
    type SaveExtraParam = ();
    
    fn restore(&self, mut q: <Self::RestoreQuery<'_> as WorldQuery>::Item<'_>, _extra: &mut StaticSystemParam<()>) {
        *q.0 = self.transform;
        *q.1 = self.velocity;
    }
    
    fn save(q: <Self::SaveQuery<'_> as WorldQuery>::Item<'_>, _extra: &mut StaticSystemParam<()>) -> Self {
        PhysicsBundle {
            transform: *q.0,
            velocity: *q.1
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct State(pub PhysicsBundle, pub EntityType);

#[derive(Serialize, Deserialize)]
pub struct States {
    pub last_frame: u64,    //maybe this type should be defined by bevy_gravirollback
    pub frame_0_time: Duration,
}

/// Snapshot of one game frame
#[derive(Serialize, Deserialize, Clone)]
pub struct Snapshot {
    /// States of all Rollback entities
    pub states: HashMap<RollbackID, State>,
    /// Inputs of this frame, they will influence the next frame
    pub inputs: Inputs,
}

#[derive(Event)]
pub struct UpdateStateEvent<S> {
    pub frame: u64,
    pub id: RollbackID,
    pub state: S,
}

pub fn clear_inputs(
    snapshot_info: Res<SnapshotInfo>,
    mut inputs: ResMut<Rollback<Inputs>>,
) {
    let index = snapshot_info.index(snapshot_info.last);
    inputs.0[index].0.clear();
}

//pub struct FutureUpdates(Vec<>);

pub fn handle_update_state_event(
    mut events: EventReader<UpdateStateEvent<State>>,
    mut snapshot_info: ResMut<SnapshotInfo>,
    rollback_map: Res<RollbackMap>,
    mut query: Query<&mut Rollback<PhysicsBundle>>,
    mut commands: Commands,
) {
    for UpdateStateEvent { frame, id, state } in events.read() {
        let frame = *frame;
        let update = frame<snapshot_info.last;
        if frame>snapshot_info.last {
            warn!("future update event frame {frame} last {}", snapshot_info.last);
            continue;
        }

        let index = snapshot_info.index(frame);
        let snapshot = &mut snapshot_info.snapshots[index];
        if snapshot.frame == frame {
            //insert this state
            snapshot.modified |= update;
            //println!("update_state_event id {id:?}");
            if let Some(&entity) = rollback_map.0.get(id) {
                //println!("update_state_event updating");
                let mut physics_bundle = query.get_mut(entity).expect("this entity should exist");
                physics_bundle.0[index].transform = state.0.transform;
                physics_bundle.0[index].velocity = state.0.velocity;
            }else{
                println!("update_state_event spawning id {id:?}");
                match state.1 {
                    EntityType::Player(player) => commands.add(spawn3(crate::player::make_player(SpawnPlayer {
                        player,
                        rollback: *id,
                        transform: state.0.transform,
                        velocity: state.0.velocity,
                        index: Some(index),
                    }))),
                    EntityType::Bullet => commands.add(spawn3(crate::bullet::make_bullet(SpawnBullet {
                        rollback: *id,
                        transform: state.0.transform,
                        velocity: state.0.velocity,
                        index: Some(index),
                    }))),
                }
            }
            
        }else{
            //too old frame
            println!("update_state_event too old frame");
        }
    }
}
