use super::EntityType;
use crate::player::{Player, SpawnPlayer};
use crate::input::Input;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy::ecs::query::WorldQuery;
use bevy::utils::{HashMap, Entry};
use circular_buffer::CircularBuffer;
use serde::{Serialize, Deserialize};

use std::time::SystemTime;

/// For temporary storage of future of super past snapshots
#[derive(Resource)]
pub struct FuturePastSnapshot<S: Restore> {
    /// The stored Snapshot
    pub snapshot: Snapshot<S>,
    /// The frame number of the Snapshot
    pub frame: u64,
}

fn serialize<Ser: serde::Serializer, S: Restore + Serialize>(data: &CircularBuffer<SNAPSHOT_LEN, Snapshot<S>>, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
    let vec = std::collections::VecDeque::from_iter(data.iter().map(|x| x.clone()));
    vec.serialize(serializer)
}
fn deserialize<'de, D: serde::Deserializer<'de>, S: Restore + Deserialize<'de>>(deserializer: D) -> Result<CircularBuffer<SNAPSHOT_LEN, Snapshot<S>>, D::Error> {
    let vec = std::collections::VecDeque::deserialize(deserializer)?;
    Ok(CircularBuffer::from_iter(vec.into_iter()))
}

/// Number of frames to the past that will be saved for doing rollback
pub const SNAPSHOT_LEN: usize = 10;

/// Snapshot storage
#[derive(Resource, Reflect, Serialize, Deserialize, Default, Clone)]
pub struct Snapshots<S: Restore> {
    /// Circular buffer of Snapshots. The last (current) Snapshot is in the back.
    /// Old snapshots get overwritten by new ones.
    #[serde(serialize_with = "serialize")]
    #[serde(deserialize_with = "deserialize")]
    #[serde(bound(serialize = "S: Serialize", deserialize = "S: Deserialize<'de>"))]
    #[reflect(ignore)]
    pub buffer: CircularBuffer<SNAPSHOT_LEN, Snapshot<S>>,
    /// The last Snapshot (frame) number - current frame
    pub frame: u64,
    /// The last Snapshot (frame) time in miliseconds since UNIX_EPOCH
    pub last_frame_time: u128,
}

/// Snapshot of one game frame
#[derive(Serialize, Deserialize, Clone)]
pub struct Snapshot<S: Restore> {
    /// States of all Rollback entities
    pub states: HashMap<Rollback, State<S>>,
    /// Inputs of this frame, they will influence the next frame
    pub inputs: Inputs,
    pub modified: bool,
}

impl<S: Restore> Snapshot<S> {
    fn restore(&self, world: &mut World) {
        self.restore_inputs(world);
        self.restore_state(world);
    }
    fn restore_inputs(&self, world: &mut World) {
        world.insert_resource(self.inputs.clone());
    }
    fn restore_state(&self, world: &mut World) {
        //println!("restoring state");
        let mut query = world.query::<(Entity, &Rollback, S::WriteQuery<'_>)>();
        let mut for_delete = Vec::new();
        let mut remaining_states = self.states.clone();
        for (ent,&r,data) in query.iter_mut(world) {
            if let Some(state2) = remaining_states.remove(&r) {
                //println!("restoring entity {ent:?} rollback {}",r.0);
                state2.state.restore(data)
            }else{
                //println!("deleting entity {ent:?} rollback {}",r.0);
                for_delete.push(ent);
            }
        }
        for entity in for_delete {
            world.entity_mut(entity).despawn_recursive();
        }
        for (r,state) in remaining_states {
            state.state.spawn(r, world);
        }
    }
    fn save_inputs(&mut self, world: &mut World) {
        self.inputs = world.resource::<Inputs>().clone();
    }
    fn save_state(&mut self, world: &mut World) {   //TODO: this function could be possibly different on Client/Server?
        let mut remaining = self.states.clone();    //TODO: use HashSet instead
        for (&r,state_for_save) in
        world.query::<(&Rollback, S::ReadQuery<'_>)>().iter(&world) {
            remaining.remove(&r);
            self.states.entry(r).and_modify(|state| {
                if !state.fixed {
                    state.state = S::save(&state_for_save);
                }
            }).or_insert(State{
                fixed: false,
                state: S::save(&state_for_save),
            });
        }
        //delete state from self, when world does not contain the rollback id, except when state.fixed
        for (r,_state) in remaining {
            let Entry::Occupied(e) = self.states.entry(r) else{panic!("remaining is a copy of self.states")};
            if !e.get().fixed {      //TODO: is this correct?
                e.remove();
            }
        }
    }
}
impl<S: Restore> Default for Snapshot<S> {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            inputs: Inputs::default(),
            modified: false,
        }
    }
}

pub struct SnapshotRef<'a, S: Restore> {
    pub states: &'a mut HashMap<Rollback, State<S>>,
    pub inputs: &'a mut Inputs,
    pub modified: &'a mut bool,
}
impl<'a, S: Restore> SnapshotRef<'a, S> {
    pub fn clone(&self) -> Snapshot<S> {
        Snapshot {
            states: self.states.clone(),
            inputs: self.inputs.clone(),
            modified: self.modified.clone(),
        }
    }
}

pub enum SnapshotType<'a, S: Restore> {
    SuperPast,
    Past(SnapshotRef<'a, S>),
    Now(SnapshotRef<'a, S>),
    Future {
        now: SnapshotRef<'a, S>
    }
}

impl<'a, S: Restore> SnapshotRef<'a, S> {
    pub fn new(
        now: u64, frame: u64,
        snapshots: &'a mut Snapshots<S>,
        now_inputs: &'a mut Inputs
    ) -> SnapshotType<'a, S> {
        let Some(offset) = now.checked_sub(frame).map(|x| x as usize) else{
            println!("future frame {frame} now {now}");
            let now_snapshot = snapshots.buffer.back_mut().expect("should contain at least one Snapshot");
            return SnapshotType::Future {
                now: Self {
                    states: &mut now_snapshot.states,
                    inputs: now_inputs,
                    modified: &mut now_snapshot.modified,
                }
            }
        };
        //now>=frame
        let Some(index) = (SNAPSHOT_LEN-1).checked_sub(offset) else{
            println!("super past frame {frame} offset {offset} now {now}");
            return SnapshotType::SuperPast
        };
        let snapshot = snapshots.buffer.get_mut(index).expect("the index calculation was checked");
        let states = &mut snapshot.states;
        let modified = &mut snapshot.modified;

        if frame==now {
            SnapshotType::Now(Self {
                states,
                inputs: now_inputs,
                modified,
            })
        }else if now>frame {
            SnapshotType::Past(Self {
                states,
                inputs: &mut snapshot.inputs,
                modified,
            })
        }else{ unreachable!() }
    }
}

/// Saved state of one Rollback entity
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
//#[serde(bound = "S: Restore<'de>")]
pub struct State<S: Restore> {
    /// When false, the Client will recompute this State when past Inputs get updated.
    /// When the Server sends corrections of the State to the Client, this flag will
    /// be set such that it wont be overwritten by the Client.
    pub fixed: bool,
    ///The saved state
    pub state: S,
}

pub trait Restore: Reflect + PartialEq + Eq + Clone + Copy {
    type WriteQuery<'a>: WorldQuery;
    type ReadQuery<'a>: WorldQuery;
    //type Data<'a> = <Self::Query<'a> as WorldQuery>::Item<'a>;    //TODO: feature associated type defaults
    fn spawn(&self, rollback: Rollback, world: &mut World);
    fn restore(&self, data: /*Self::Data<'_>*/ <Self::WriteQuery<'_> as WorldQuery>::Item<'_>);
    fn save(state_for_save: /*Self::Data<'_>*/ &<<Self::ReadQuery<'_> as WorldQuery>::ReadOnly as WorldQuery>::Item<'_>) -> Self;
}

/// My custom State
#[derive(Reflect, Serialize, Deserialize, Clone, Copy)]
pub struct MyState {
    pub entity: EntityType,
    pub transform: Transform,
    pub velocity: Velocity,
}
impl Restore for MyState {
    type WriteQuery<'a> = (&'a EntityType, &'a mut Transform, &'a mut Velocity);
    type ReadQuery<'a> = (&'a EntityType, &'a Transform, &'a Velocity);
    fn spawn(&self, rollback: Rollback, world: &mut World) {
        match self.entity {
            EntityType::Player(player) => {
                let event = SpawnPlayer {
                    player,
                    rollback,
                    transform: self.transform,
                };
                let entity = world.spawn_empty().id();
                crate::player::make_player(event, Some(entity))(world);
                *world.get_mut::<Velocity>(entity).expect("the entity was just spawned") = self.velocity;
            },
            EntityType::Bullet => {
                let event = crate::bullet::SpawnBullet {
                    rollback,
                    transform: self.transform,
                    velocity: self.velocity,
                };
                crate::bullet::make_bullet(event, None)(world);
            }
        };
    }
    fn restore(&self, mut data: /*Self::Data<'_>*/ <Self::WriteQuery<'_> as WorldQuery>::Item<'_>) {
        assert!(*data.0 == self.entity);
        *data.1 = self.transform;
        *data.2 = self.velocity;
    }
    fn save(data: /*Self::Data<'_>*/ &<Self::ReadQuery<'_> as WorldQuery>::Item<'_>) -> Self {
        Self {
            entity: *data.0,
            transform: *data.1,
            velocity: *data.2,
        }
    }
}
impl PartialEq for MyState {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity && self.transform == other.transform && self.velocity == other.velocity
    }
}
impl Eq for MyState {}

#[derive(Resource, Reflect, Default, Serialize, Deserialize, Clone)]
#[reflect(Resource)]
pub struct Inputs(pub HashMap<Player, Input>);

/// Entities with this component will be afected by rollback
#[derive(Component, Reflect, FromReflect, Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy)]
pub struct Rollback(pub u64);

pub enum RollbackStages {
    CorePreUpdate,
    CoreUpdate,
    PhysicsStagesSyncBackend,
    PhysicsStagesStepSimulation,
    PhysicsStagesWriteback,
    CorePostUpdate,
    PhysicsStagesDetectDespawn,
    //CoreLast,
    /// DO NOT USE! The total amount of variants of this enum, used when creating Vec<SystemStage>
    TotalDoNotUse
}

#[derive(Resource)]
pub struct RollbackStagesStorage(Vec<SystemStage>);

impl RollbackStagesStorage {
    pub fn new() -> Self {
        Self((0..RollbackStages::TotalDoNotUse as usize).map(|_| SystemStage::parallel()).collect())
    }
    pub fn get(&mut self, label: RollbackStages) -> &mut SystemStage {
        &mut self.0[label as usize]
    }
}

pub fn run_update(world: &mut World) {
    world.resource_scope(|world, mut stages:Mut<RollbackStagesStorage>| {
        for stage in 0..RollbackStages::TotalDoNotUse as usize {
            //stages.0[stage].apply_buffers(world);   //TODO: is this correct? or put it after run()? or is it called automatically in run()?
            stages.0[stage].run(world);
        }
    });
}

//gets called after normal systems handle networking and player inputs and modify Snapshots
pub fn rollback_schedule<S: Restore>(world: &mut World) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH).expect("since UNIX_EPOCH")
        .as_millis();
    let mut future = world.remove_resource::<FuturePastSnapshot<S>>();
    
    world.resource_scope(|world, mut snapshots: Mut<Snapshots<S>>| {
        let _target_frame = future.as_ref().map(|x| x.frame).unwrap_or(snapshots.frame)+1;
        while /*snapshots.frame<_target_frame ||*/ snapshots.last_frame_time<now {
            //println!("rollback update now {now} target time {} frame {} target frame {}",snapshots.last_frame_time,snapshots.frame,_target_frame);
            //insert the future snapshot when the correct frame is reached
            if let Some(f) = future.as_ref() {
                if f.frame==snapshots.frame {
                    let snapshot = future.take().expect("future contains value").snapshot;
                    if !snapshot.states.is_empty() {
                        snapshot.restore_state(world);
                    }
                    if !snapshot.inputs.0.is_empty() {
                        snapshot.restore_inputs(world);
                    }
                }
            }

            //save inputs of current frame
            snapshots.buffer.back_mut().expect("should contain at least one Snapshot").save_inputs(world);
            //prepare new empty snapshot - the next frame
            snapshots.buffer.push_back(Snapshot::default());
            snapshots.frame += 1;
            snapshots.last_frame_time += 1000/60; //TODO: move into constant

            let mut needs_restore = false;  //TODO: optimize -> instead store last loaded snapshot and do not restore when it is already loaded
            let len  = snapshots.buffer.len();
            for i in 0..len-2 {
                let snapshot = snapshots.buffer.get_mut(i)
                    .expect("index i is always < length-2");
                
                if snapshot.modified {
                    //println!("rollback modified index {i}");
                    snapshot.modified = false;
                    needs_restore = true;

                    snapshot.restore(world);
                    run_update(world);
                    let next_snapshot = snapshots.buffer.get_mut(i+1)
                        .expect("index i is always < length-2");
                    next_snapshot.modified = true;
                    next_snapshot.save_state(world);
                }
            }

            let snapshot = snapshots.buffer.get_mut(len-2)
                .expect("second last snapshot should exist");

            //println!("last frame index {} modified {} needs restore {needs_restore}",len-2,snapshot.modified);
            snapshot.modified = false;
            if needs_restore {  //TODO: can this be instead checked by snapshot.modified?
                snapshot.restore(world);
            }
            run_update(world);
            let next_snapshot = snapshots.buffer.back_mut()
                .expect("should contain at least one Snapshot");
            next_snapshot.modified = false;
            next_snapshot.save_state(world);
        }
    });
    if let Some(future) = future.take() {
        world.insert_resource(future);
    }
}