use std::collections::HashMap;
use std::rc::Rc;
use arrayvec::ArrayVec;

use geometry::{Number, Vector, GridIsomorphism};
use machine_data::{Material, MachineType, MachineTypeTrait, MachineMapState, StatefulMachine};
use graph_algorithms::MapFuture;
use modules::{Module, ModuleMachine};

pub const MAX_COMPONENTS: usize = 256;

pub type Machines = ArrayVec <[StatefulMachine; MAX_COMPONENTS]>;


#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Map {
  pub machines: Machines,
  pub last_change_time: Number,  
}

#[derive (Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Game {
  pub map: Map,
  pub future: MapFuture,
  pub inventory_before_last_change: HashMap <Material, Number>,
}


pub trait MapQuery {
  type Output;
  fn enters_module (&self, map_state: &MachineMapState, module_machine: &mut ModuleMachine)->bool;
  fn apply (self, machines: &Machines)-> Self::Output;
}
pub trait MapEdit {
  type Output;
  fn enters_module (&self, map_state: &MachineMapState, module_machine: &mut ModuleMachine)->bool;
  fn apply (self, machines: &mut Machines)-> Self::Output;
}
pub trait QueryMap {
  fn query_map<Q: MapQuery>(&self, now: Number, query: Q)-> Q::Output;
}
pub trait EditMap {
  fn edit_map<E: MapEdit>(&mut self, now: Number, edit: E)-> E::Output;
}
impl <Q: MapQuery> MapEdit for Q {
  type Output = Q::Output;
  fn enters_module (&self, map_state: &MachineMapState, module_machine: &mut ModuleMachine)->bool {self.enters_module(map_state, module_machine)}
  fn apply (self, machines: &mut Machines)-> Self::Output {self.apply(machines)}
}

pub struct QuerySmallestModuleContainingSquare <F> {
  position: Vector,
  radius: Number,
  callback: F
}
pub struct EditSmallestModuleContainingSquare <F> {
  position: Vector,
  radius: Number,
  callback: F
}

impl<F: FnOnce(&Machines)-> R, R> MapQuery for QuerySmallestModuleContainingSquare<F> {
  type Output = R;
  fn enters_module (&self, map_state: &MachineMapState, module_machine: &mut ModuleMachine)->bool {
    let relative_position = self.position - map_state.position.translation;
    let available_radius = module_machine.module.module_type.inner_radius - self.radius;
    relative_position[0].abs() <= available_radius && relative_position[1].abs() <= available_radius
  }
  fn apply (self, machines: &Machines)-> Self::Output {
    (self.callback)(machines)
  }
}
impl<F: FnOnce(&mut Machines)-> R, R> MapEdit for EditSmallestModuleContainingSquare<F> {
  type Output = R;
  fn enters_module (&self, map_state: &MachineMapState, module_machine: &mut ModuleMachine)->bool {
    let relative_position = self.position - map_state.position.translation;
    let available_radius = module_machine.module.module_type.inner_radius - self.radius;
    relative_position[0].abs() <= available_radius && relative_position[1].abs() <= available_radius
  }
  fn apply (self, machines: &mut Machines)-> Self::Output {
    (self.callback)(machines)
  }
}


impl<'a> EditMap for (&'a mut Map, &'a MapFuture) {
  fn edit_map<E: MapEdit>(&mut self, now: Number, edit: E)-> E::Output {
    let (map, future) = self;
    map.last_change_time = now;
    for (machine, future) in map.machines.iter_mut().zip (& future.machines) {
      machine.materials_state = machine.machine_type.with_inputs_changed(& future.materials_state_at (now, & machine.materials_state), now, &future.inputs_at(now));
    }
    for machine in map.machines.iter_mut() {
      if let MachineType::ModuleMachine(module_machine) = &mut machine.machine_type {
        if edit.enters_module(&machine.map_state, module_machine) {
          let mut edited: Module = (*module_machine.module).clone();
          for adjusting_machine in &mut edited.map.machines {
            adjusting_machine.map_state.position *= machine.map_state.position;
          }
          let local_now = unimplemented!();
          let result = edited.edit_map(local_now, edit);
          for adjusting_machine in &mut edited.map.machines {
            adjusting_machine.map_state.position /= machine.map_state.position;
          }
          module_machine.module = Rc::new(edited);
          return result;
        }
      }
    }
    edit.apply(&mut map.machines)
  }
}

impl EditMap for Game {
  fn edit_map<E: MapEdit>(&mut self, now: Number, edit: E)-> E::Output {
    self.inventory_before_last_change = self.inventory_at (& self.future, now);
    let result = (&mut self.map, &self.future).edit_map(now, edit);
    let output_edges = self.map.output_edges();
    let ordering = self.map.topological_ordering_of_noncyclic_machines(& output_edges);
    self.future = self.map.future (& output_edges, & ordering);
    result
  }
}

impl QueryMap for Game {
  fn query_map<E: MapQuery>(&self, now: Number, query: E)-> E::Output {
    self.clone().edit_map(now, query)
  }
}


// TODO reduce duplicate code id 394342002
fn in_smallest_module<F: FnOnce(GridIsomorphism, &ArrayVec <[StatefulMachine; MAX_COMPONENTS]>)->R, R> (machines: &ArrayVec <[StatefulMachine; MAX_COMPONENTS]>, isomorphism: GridIsomorphism, (position, radius): (Vector, Number), callback: F)->R {
  for machine in machines.iter() {
    if let MachineType::ModuleMachine(module_machine) = &machine.machine_type {
      let machine_isomorphism = machine.map_state.position*isomorphism;
      let relative_position = position - machine_isomorphism.translation;
      let available_radius = module_machine.module.module_type.inner_radius - radius;
      if relative_position[0].abs() <= available_radius && relative_position[1].abs() <= available_radius {
        return in_smallest_module(&module_machine.module.map.machines, machine_isomorphism, (position, radius), callback);
      }
    }
  }
  callback (isomorphism, machines)
}
// TODO reduce duplicate code id 394342002
fn edit_in_smallest_module<F: FnOnce(GridIsomorphism, &mut ArrayVec <[StatefulMachine; MAX_COMPONENTS]>)->R, R> (machines: &mut ArrayVec <[StatefulMachine; MAX_COMPONENTS]>, isomorphism: GridIsomorphism, (position, radius): (Vector, Number), callback: F)->R {
  for machine in machines.iter_mut() {
    if let MachineType::ModuleMachine(module_machine) = &mut machine.machine_type {
      let machine_isomorphism = machine.map_state.position*isomorphism;
      let relative_position = position - machine_isomorphism.translation;
      let available_radius = module_machine.module.module_type.inner_radius - radius;
      if relative_position[0].abs() <= available_radius && relative_position[1].abs() <= available_radius {
        let mut edited: Module = (*module_machine.module).clone();
        let result = edit_in_smallest_module(&mut edited.map.machines, machine_isomorphism, (position, radius), callback);
        module_machine.module = Rc::new(edited);
        return result;
      }
    }
  }
  callback (isomorphism, machines)
}

  