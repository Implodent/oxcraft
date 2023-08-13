//! Collections of complimentary systems and resources.
//!
//! Systems and resources can be composed together using the [`Plugin`] builder.
//! The resulting plugin can then be instantiated with
//! [`crate::World::with_plugin`].
use std::{any::TypeId, future::Future};

use dagga::Node;

use crate::{
    resource_manager::LoanManager,
    // schedule::Dependency,
    system::{AsyncSystemFuture, ShouldContinue, System},
    world::Facade,
    CanFetch,
    IsResource,
    LazyResource,
    World,
};

pub struct SyncSystemWithDeps(pub Node<System, TypeId>);

impl SyncSystemWithDeps {
    pub fn new<T, F>(
        name: &str,
        system: F,
        after_deps: Option<&[&str]>,
        before_deps: Option<&[&str]>,
    ) -> Self
    where
        F: FnMut(T) -> anyhow::Result<ShouldContinue> + Send + Sync + 'static,
        T: CanFetch + Send + Sync + 'static,
    {
        let run_after = after_deps.unwrap_or_default();
        let run_before = before_deps.unwrap_or_default();
        SyncSystemWithDeps(System::node(name, system, &run_after, &run_before))
    }
}

pub struct LazyAsyncSystem {
    pub name: String,
    pub make_future: Box<dyn FnOnce(Facade) -> AsyncSystemFuture>,
}

/// A builder of resource requirements and systems.
///
/// A plugin can contain duplicate entries of resources and systems. At the time
/// when the plugin is loaded into the world with
/// [`World::with_plugin`](crate::World::with_plugin), all resources and systems
/// will be created once and will be unique.
#[derive(Default)]
pub struct WorldBuilder {
    pub(crate) resources: Vec<LazyResource>,
    pub(crate) sync_systems: Vec<SyncSystemWithDeps>,
    pub(crate) async_systems: Vec<LazyAsyncSystem>,
}

pub trait Plugin {
    fn apply(self, builder: &mut WorldBuilder);
}

impl Plugin for WorldBuilder {
    fn apply(self, builder: &mut WorldBuilder) {
        builder.resources.extend(self.resources);
        builder.sync_systems.extend(self.sync_systems);
        builder.async_systems.extend(self.async_systems);
    }
}

impl WorldBuilder {
    pub fn with_plugin(&mut self, plug: impl Plugin) -> &mut Self {
        plug.apply(self);
        self
    }

    /// Add a dependency on a resource that may be created using other existing
    /// and fetchable resources.
    ///
    /// If this resource does not already exist in the world at the time this
    /// plugin is instantiated, it will be inserted into the
    /// [`World`](crate::World), if possible - otherwise instantiation will
    /// err.
    pub fn with_resource<S: CanFetch, T: IsResource>(
        &mut self,
        create: impl FnOnce(S) -> anyhow::Result<T> + 'static,
    ) -> &mut Self {
        self.resources
            .push(LazyResource::new(move |loans: &mut LoanManager| {
                let s: S = S::construct(loans)?;
                let t: T = create(s)?;
                Ok(t)
            }));
        self
    }

    /// Add a system to the plugin.
    pub fn with_system<T: CanFetch + Send + Sync + 'static>(
        &mut self,
        name: &str,
        system: impl FnMut(T) -> anyhow::Result<ShouldContinue> + Send + Sync + 'static,
        // other systems this system must run after
        after_deps: &[&str],
        // other systems this system must run before
        before_deps: &[&str],
    ) -> &mut Self {
        let after_deps = if after_deps.is_empty() {
            None
        } else {
            Some(after_deps)
        };
        let before_deps = if before_deps.is_empty() {
            None
        } else {
            Some(before_deps)
        };
        self.sync_systems.push(SyncSystemWithDeps::new(
            name,
            system,
            after_deps,
            before_deps,
        ));
        self.with_plugin(T::plugin())
    }

    pub fn with_async<Fut>(
        &mut self,
        name: impl Into<String>,
        system: impl FnOnce(Facade) -> Fut + 'static,
    ) -> &mut Self
    where
        Fut: Future<Output = anyhow::Result<()>> + Send + Sync + 'static,
    {
        let name = name.into();
        self.async_systems.push(LazyAsyncSystem {
            name,
            make_future: Box::new(move |facade| Box::pin(system(facade))),
        });
        self
    }

    /// Build a world from this plugin.
    pub fn build(self) -> anyhow::Result<World> {
        let mut world = World::default();
        world.with_plugin(self)?;
        Ok(world)
    }
}

#[cfg(test)]
mod test {
    use crate::{self as apecs, storage::*, system::*, world::World, Read, WorldBuilder};

    #[derive(Default)]
    pub struct Number(u32);

    #[derive(apecs_derive::CanFetch)]
    pub struct MyData(Query<(&'static mut String, &'static mut usize)>);

    #[test]
    fn sanity() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .try_init();

        fn my_system(data: MyData) -> anyhow::Result<ShouldContinue> {
            for (_, n) in data.0.query().iter_mut() {
                **n = **n + 1;
            }

            ok()
        }

        let mut world = World::default();
        world.with_system("my_system", my_system).unwrap();

        let _data = world.fetch::<MyData>().unwrap();
    }

    #[test]
    fn can_build_dependent_resources() {
        let plugin = WorldBuilder::default()
            .with_resource::<Read<Number>, bool>(|num| Ok(num.inner().0 == 0));
        let mut world = World::default();
        world.with_plugin(plugin).unwrap();

        let boolean = world.resource::<bool>().unwrap();
        assert!(boolean);
    }
}
