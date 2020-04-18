use crate::SoundStorage;
use amethyst::core::bundle::SystemBundle;
use amethyst::ecs::*;
use amethyst::error::Error;
use amethyst::{
    assets::AssetStorage,
    audio::{output::Output, Source, SourceHandle},
    core::transform::{components::Parent, Transform},
};
use nalgebra::geometry::{Isometry2, Point2, Point3, UnitQuaternion};
use nalgebra::{RealField, Vector2};
use ncollide2d::pipeline::narrow_phase::ContactEvent;
use nphysics2d::force_generator::DefaultForceGeneratorSet;
use nphysics2d::joint::DefaultJointConstraintSet;
use nphysics2d::math::{Force, ForceType};
use nphysics2d::object::*;
use nphysics2d::world::{
    DefaultGeometricalWorld, DefaultMechanicalWorld, GeometricalWorld, MechanicalWorld,
};

#[derive(Component)]
#[storage(VecStorage)]
pub struct PhysicsDesc {
    body: RigidBodyDesc<f32>,
    collider: ColliderDesc<f32>,
}

impl PhysicsDesc {
    pub fn new(body: RigidBodyDesc<f32>, collider: ColliderDesc<f32>) -> Self {
        PhysicsDesc { body, collider }
    }
}

#[derive(Component)]
#[storage(VecStorage)]
pub struct AttachedSensor {
    collider: ColliderDesc<f32>,
    handle: Option<(DefaultBodyHandle, DefaultColliderHandle)>,
}

impl AttachedSensor {
    pub fn new(collider: ColliderDesc<f32>) -> Self {
        AttachedSensor {
            collider,
            handle: None,
        }
    }
    pub fn set_handle(&mut self, handle: (DefaultBodyHandle, DefaultColliderHandle)) {
        self.handle = Some(handle);
    }
}

#[derive(Component, Debug, Clone)]
#[storage(VecStorage)]
pub struct PhysicsHandle {
    body: Option<DefaultBodyHandle>,
    collider: Option<DefaultColliderHandle>,
    fresh: bool,
}

impl PhysicsHandle {
    pub fn new(b: DefaultBodyHandle, c: DefaultColliderHandle) -> Self {
        Self {
            body: Some(b),
            collider: Some(c),
            fresh: true,
        }
    }
}

pub struct Physics<N: RealField> {
    pub geo_world: DefaultGeometricalWorld<N>,
    pub mech_world: DefaultMechanicalWorld<N>,
    pub bodies: DefaultBodySet<N>,
    pub colliders: DefaultColliderSet<N>,
    pub joint_constraints: DefaultJointConstraintSet<N>,
    pub force_generators: DefaultForceGeneratorSet<N>,
}

impl<N: RealField> Physics<N> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self) {
        self.mech_world.step(
            &mut self.geo_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joint_constraints,
            &mut self.force_generators,
        )
    }

    pub fn spawn(
        &mut self,
        body_desc: &RigidBodyDesc<N>,
        collider_desc: &ColliderDesc<N>,
    ) -> (DefaultBodyHandle, DefaultColliderHandle) {
        let handle = self.bodies.insert(body_desc.build());
        let collider_handle = self
            .colliders
            .insert(collider_desc.build(BodyPartHandle(handle, 0)));
        (handle, collider_handle)
    }

    pub fn add_child_collider(
        &mut self,
        parent_handle: &PhysicsHandle,
        collider_desc: &ColliderDesc<N>,
    ) -> DefaultColliderHandle {
        if let Some(handle) = parent_handle.body {
            let collider_handle = self
                .colliders
                .insert(collider_desc.build(BodyPartHandle(handle, 0)));
            collider_handle
        } else {
            panic!("No parent!");
        }
    }

    pub fn get_position(&self, handle: &PhysicsHandle) -> Option<Isometry2<N>> {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body(handle) {
                Some(rigid_body.position().clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn set_location(&mut self, handle: &PhysicsHandle, x: N, y: N) {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body_mut(handle) {
                rigid_body.set_position(Isometry2::new(
                    Vector2::new(x, y),
                    rigid_body.position().rotation.angle(),
                ));
            }
        }
    }

    pub fn set_sensor_position(&mut self, sensor: &AttachedSensor, x: N, y: N) {
        if let Some(handle) = sensor.handle {
            if let (Some(parent), Some(collider)) = (
                self.bodies.rigid_body(handle.0),
                self.colliders.get_mut(handle.1),
            ) {
                collider.set_position(Isometry2::new(
                    parent.position().translation.vector + Vector2::new(x, y),
                    collider.position().rotation.angle(),
                ));
            }
        }
    }

    pub fn set_rotation(&mut self, handle: &PhysicsHandle, radians: N) {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body_mut(handle) {
                rigid_body.set_position(Isometry2::new(
                    rigid_body.position().translation.vector,
                    radians,
                ));
            }
        }
    }

    pub fn set_velocity(&mut self, handle: &PhysicsHandle, vec: Vector2<N>) {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body_mut(handle) {
                rigid_body.set_linear_velocity(vec);
            }
        }
    }

    pub fn apply_impulse(&mut self, handle: &PhysicsHandle, vec: Vector2<N>) {
        if let Some(handle) = handle.body {
            if let Some(body) = self.bodies.get_mut(handle) {
                body.apply_force(0, &Force::linear(vec), ForceType::Impulse, true)
            }
        }
    }

    pub fn get_body_entity(&self, handle: DefaultBodyHandle) -> Option<&Entity> {
        if let Some(rigid_body) = self.bodies.rigid_body(handle) {
            if let Some(m_entity) = rigid_body.user_data() {
                m_entity.downcast_ref::<Entity>()
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_collider_entity(&self, handle: DefaultColliderHandle) -> Option<&Entity> {
        if let Some(collider) = self.colliders.get(handle) {
            if let Some(m_entity) = collider.user_data() {
                m_entity.downcast_ref::<Entity>()
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<N: RealField> Default for Physics<N> {
    fn default() -> Self {
        Self {
            mech_world: DefaultMechanicalWorld::new(Vector2::new(N::zero(), N::zero())),
            geo_world: DefaultGeometricalWorld::new(),
            bodies: DefaultBodySet::new(),
            colliders: DefaultColliderSet::new(),
            joint_constraints: DefaultJointConstraintSet::new(),
            force_generators: DefaultForceGeneratorSet::new(),
        }
    }
}

struct PhysicsSystem;

impl<'s> System<'s> for PhysicsSystem {
    type SystemData = (
        Write<'s, Physics<f32>>,
        ReadStorage<'s, PhysicsHandle>,
        WriteStorage<'s, Transform>,
    );

    fn run(&mut self, (mut physics, handles, mut transforms): Self::SystemData) {
        physics.step();
        for (handle, transform) in (&handles, &mut transforms).join() {
            if let Some(position) = physics.get_position(handle) {
                let x = position.translation.x;
                let y = position.translation.y;
                let rotation_2d = position.rotation.angle();
                transform.set_translation_x(x as f32);
                transform.set_translation_y(y as f32);
                transform.set_rotation_2d(rotation_2d as f32);
            }
        }
    }
}

struct PhysicsSpawningSystem;

impl<'s> System<'s> for PhysicsSpawningSystem {
    type SystemData = (
        Write<'s, Physics<f32>>,
        ReadStorage<'s, PhysicsDesc>,
        WriteStorage<'s, AttachedSensor>,
        ReadStorage<'s, Parent>,
        WriteStorage<'s, PhysicsHandle>,
        WriteStorage<'s, Transform>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (mut physics, descs, mut attached, parent, mut handles, mut transforms, entities): Self::SystemData,
    ) {
        for (entity, desc) in (&entities, &descs).join() {
            if !handles.contains(entity) {
                let (handle, collider_handle) = physics.spawn(&desc.body, &desc.collider);
                let phys_handle = PhysicsHandle::new(handle, collider_handle);
                if let Some(transform) = transforms.get(entity) {
                    let translation = transform.translation();
                    physics.set_location(&phys_handle, translation.x as f32, translation.y as f32);
                } else {
                    transforms.insert(entity, Transform::default());
                }
                handles.insert(entity, phys_handle);
            }
            for (child_entity, parent, attached) in (&entities, &parent, &mut attached).join() {
                if parent.entity == entity {
                    if let Some(handle) = handles.get(entity) {
                        if attached.handle.is_none() {
                            println!("Adding sensor!");
                            let sensor_handle =
                                physics.add_child_collider(handle, &attached.collider);
                            attached.set_handle((handle.body.unwrap(), sensor_handle));
                        }
                    }
                }
            }
        }
        for (entity, handle) in (&entities, &mut handles).join() {
            if handle.fresh {
                if let Some(body_handle) = handle.body {
                    if let Some(body) = physics.bodies.rigid_body_mut(body_handle) {
                        body.set_user_data(Some(Box::new(entity)));
                    }
                }
                if let Some(coll_handle) = handle.collider {
                    if let Some(collider) = physics.colliders.get_mut(coll_handle) {
                        collider.set_user_data(Some(Box::new(entity)));
                    }
                }
                handle.fresh = false;
            }
        }
    }
}

struct PhysicsDeletionSystem;

impl<'s> System<'s> for PhysicsDeletionSystem {
    type SystemData = (Write<'s, Physics<f32>>, Entities<'s>);

    fn run(&mut self, (mut physics, entities): Self::SystemData) {
        let mut bodies_to_remove = Vec::new();
        let mut colliders_to_remove = Vec::new();
        for (handle, body) in physics.bodies.iter() {
            if let Some(entity) = physics.get_body_entity(handle) {
                if !entities.is_alive(*entity) {
                    bodies_to_remove.push(handle);
                }
            }
        }
        for (handle, collider) in physics.colliders.iter() {
            if let Some(entity) = physics.get_collider_entity(handle) {
                if !entities.is_alive(*entity) {
                    colliders_to_remove.push(handle);
                }
            }
        }
        for handle in bodies_to_remove.iter() {
            physics.bodies.remove(*handle);
        }
        for handle in colliders_to_remove.iter() {
            physics.colliders.remove(*handle);
        }
    }
}

struct BounceSystem;

impl<'s> System<'s> for BounceSystem {
    type SystemData = (
        Write<'s, Physics<f32>>,
        Option<Read<'s, SoundStorage>>,
        Option<Read<'s, Output>>,
        Read<'s, AssetStorage<Source>>,
    );

    fn run(&mut self, (mut physics, storage, output, sources): Self::SystemData) {
        let mut contacts = Vec::new();
        for (col_handle1, collider1, col_handle2, collider2, _, manifold) in
            physics.geo_world.contact_pairs(&physics.colliders, true)
        {
            if let Some(tracked_contact) = manifold.deepest_contact() {
                let BodyPartHandle(handle1, _) = collider1.body_part(0);
                let BodyPartHandle(handle2, _) = collider2.body_part(0);
                contacts.push((handle1.clone(), handle2.clone(), tracked_contact.clone()));
            }
        }
        for (handle1, handle2, tracked_contact) in contacts.iter() {
            if let Some(ref output) = output.as_ref() {
                if let Some(ref sounds) = storage.as_ref() {
                    if let Some(sound) = sources.get(&sounds.bounce_wav.clone()) {
                        output.play_once(sound, 1.0);
                    }
                }
            }
        }
    }
}

pub struct PhysicsBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for PhysicsBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(PhysicsSpawningSystem, "physics_spawn", &[]);
        dispatcher.add(PhysicsSystem, "physics", &["physics_spawn"]);
        dispatcher.add(PhysicsDeletionSystem, "physics_delete", &[]);
        dispatcher.add(BounceSystem, "bounce", &[]);
        Ok(())
    }
}
