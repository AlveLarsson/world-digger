extern crate amethyst;
extern crate amethyst_rhusics;
extern crate rhusics_core;
extern crate rhusics_ecs;
extern crate collision;

use amethyst::{Application, Error, State, Trans};
use amethyst::assets::{Loader,AssetStorage};
use amethyst::config::Config;
use amethyst::controls::{FlyControlTag,FlyControlBundle};
use amethyst::core::frame_limiter::FrameRateLimitStrategy;
use amethyst::core::transform::{GlobalTransform, Transform, TransformBundle};
use amethyst::ecs::{World,VecStorage,Component,Fetch,Entity};
use amethyst::input::InputBundle;
use amethyst::renderer::{AmbientColor, Camera, DisplayConfig, DrawShaded, ElementState, Event,
                         KeyboardInput, Material, MaterialDefaults, MeshHandle, ObjFormat,
                         Pipeline, PosNormTex, Projection, RenderBundle, Rgba, Stage,
                         VirtualKeyCode, WindowEvent,Texture};
use amethyst::shrev::EventChannel;

use amethyst_rhusics::{time_sync, DefaultBasicPhysicsBundle3};
use collision::Aabb3;
use collision::primitive::{Primitive3,Cuboid};
use rhusics_core::{CollisionShape, RigidBody,Collider,ContactEvent};
use rhusics_ecs::WithRigidBody;
use rhusics_ecs::physics3d::{BodyPose3, CollisionMode, CollisionStrategy, Mass3};
use amethyst::core::cgmath::{Deg, Array, Basis3,Basis2, One, Point3, Quaternion, Vector3};

pub type Shape = CollisionShape<Primitive3<f32>, BodyPose3<f32>, Aabb3<f32>, ObjectType>;

#[repr(u8)]
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum ObjectType {
    Box,
}

impl Default for ObjectType {
    fn default() -> Self {
        ObjectType::Box
    }
}

impl Collider for ObjectType {
    fn should_generate_contacts(&self, other: &ObjectType) -> bool {
        self != other
    }
}

impl Component for ObjectType {
    type Storage = VecStorage<Self>;
}


struct ExampleState;

impl State for ExampleState {
    fn on_start(&mut self, world: &mut World) {
        initialise_camera(world);

        let (mut comps,cube) = {
            let mat_defaults = world.read_resource::<MaterialDefaults>().clone();

            let loader = world.read_resource::<Loader>();
            let cube = {
                let mesh_storage = world.read_resource();
                loader.load("cube.obj", ObjFormat, (), (), &mesh_storage)
            };

            let tex_storage = world.read_resource();


            let radius = 4;
            let cube_size = 1.0;

            let mut comps: Vec<(Material, Transform)> = vec![];

            for x in -radius..radius {
                for y in -radius..radius {
                    for z in -radius..radius {

                        // CUBE COLOR
                        let r_color = (x + radius) as f32 / (radius as f32 * 2.0);
                        let g_color = (y + radius) as f32 / (radius as f32 * 2.0);
                        let b_color = (z + radius) as f32 / (radius as f32 * 2.0);

                        let color = mat_from_color([r_color, g_color, b_color, 1.0], &mat_defaults, &loader, &tex_storage);
                        // CUBE COLOR END

                        let x = x as f32 * cube_size;
                        let y = y as f32 * cube_size;
                        let z = z as f32 * cube_size;
                        let mut trans = Transform::default();
                        trans.translation = Vector3::new(x, y, z);

                        comps.push((color, trans));
                    }
                }
            }
            (comps,cube)
        };

        world.register::<ObjectType>();
        world.write_resource::<EventChannel<ContactEvent<Entity, Point3<f32>>>>();

        while let Some(c) = comps.pop(){
            world
                .create_entity()
                .with(cube.clone())
                .with(c.0)
                .with(c.1)
                .with(GlobalTransform::default())
                .with_static_rigid_body(
                    Shape::new_simple_with_type(
                        CollisionStrategy::FullResolution,
                        CollisionMode::Discrete,
                        Cuboid::new(1.0, 1.0,1.0).into(),
                        ObjectType::Box,
                    ),
                    BodyPose3::new(Point3::new(0.0, 0.0,0.0), Quaternion::one()),
                    RigidBody::default(),
                    Mass3::infinite(),
                )
                .build();
        }

        world.add_resource(AmbientColor(Rgba::from([1.0; 3])));
    }

    fn handle_event(&mut self, _: &mut World, event: Event) -> Trans {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                    KeyboardInput {
                        virtual_keycode,
                        state: ElementState::Pressed,
                        ..
                    },
                    ..
                } => match virtual_keycode {
                    Some(VirtualKeyCode::Escape) => return Trans::Quit,
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
        Trans::None
    }
}

fn mat_from_color(color: [f32;4], defaults: &MaterialDefaults, loader: &Loader, tex_storage: &AssetStorage<Texture>)->Material{
    let color = loader.load_from_data(color.into(), (), &tex_storage);
    Material {
        albedo: color,
        ..defaults.0.clone()
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Could not run the example!");
        eprintln!("{}", error);
        ::std::process::exit(1);
    }
}

/// Wrapper around the main, so we can return errors easily.
fn run() -> Result<(), Error> {
    let resources_directory = format!("{}/assets", env!("CARGO_MANIFEST_DIR"));

    let display_config_path = format!(
        "{}/resources/display_config.ron",
        env!("CARGO_MANIFEST_DIR")
    );

    let display_config = DisplayConfig::load(display_config_path);

    let key_bindings_path = format!(
        "{}/resources/input.ron",
        env!("CARGO_MANIFEST_DIR")
    );

    let pipeline_builder = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([1.0, 0.6, 0.8, 1.0], 1.0)
            .with_pass(DrawShaded::<PosNormTex>::new()),
    );
    let mut game = Application::build(resources_directory, ExampleState)?
        .with_frame_limit(FrameRateLimitStrategy::Unlimited, 0)
        .with_bundle(FlyControlBundle::<String, String>::new(
            Some(String::from("move_x")),
            Some(String::from("move_y")),
            Some(String::from("move_z")),
        ).with_speed(20.0).with_sensitivity(0.3,0.3))?
        .with_bundle(TransformBundle::new().with_dep(&["fly_movement"]))?
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path),
        )?
        .with_bundle(RenderBundle::new(pipeline_builder, Some(display_config)))?
        .with_bundle(DefaultBasicPhysicsBundle3::<f32,ObjectType>::new())?
        .build()?;
    game.run();
    Ok(())
}

fn initialise_camera(world: &mut World) {
    let local = Transform::default();

    world
        .create_entity()
        .with(Camera::from(Projection::perspective(1.3, Deg(60.0))))
        .with(local)
        .with(GlobalTransform::default())
        .with(FlyControlTag)
        .build();
}