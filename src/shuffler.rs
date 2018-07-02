use bidir_map::BidirMap;
use rand::{Rng, StdRng};

#[derive(Copy, Clone, Eq, Hash, Debug)]
pub struct Destination {
    pub id: usize,
    pub destination_bytes: [u8; 4]
}

impl PartialEq for Destination {
    fn eq(&self, other: &Destination) -> bool {
        self.id == other.id
    }
}

impl Destination {
    pub fn new(id: usize, destination_bytes: [u8; 4]) -> Destination {
        Destination {
            id,
            destination_bytes
        }
    }
}

#[derive(Copy, Clone, Eq, Hash, Debug)]
pub struct Exit {
    pub id: usize,
    pub exit_addr1: usize,
    pub exit_addr2: usize,
    pub exit_type: ExitType,
    pub linked_door_id: i32
}

impl PartialEq for Exit {
    fn eq(&self, other: &Exit) -> bool {
        self.id == other.id
    }
}

impl Exit {
    pub fn new(
        id: usize,
        exit_addr1: usize,
        exit_addr2: usize,
        exit_type: ExitType,
        linked_door_id: i32
    ) -> Exit {
        Exit {
            id,
            exit_addr1,
            exit_addr2,
            exit_type,
            linked_door_id
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ExitType {
    OneWay,
    TwoWay
}

#[derive(Clone, Debug)]
pub struct Room {
    pub id: usize,
    pub one_way_entrances: Vec<Destination>,
    pub two_way_entrances: Vec<Destination>,
    pub one_way_exits: Vec<Exit>,
    pub two_way_exits: Vec<Exit>
}

impl PartialEq for Room {
    fn eq(&self, other: &Room) -> bool {
        self.id == other.id
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Door(pub Destination, pub Exit);

pub struct Shuffler {
    original_destination_exit_map: BidirMap<Destination, Exit>,
    original_links: BidirMap<Door, Door>
}

impl Shuffler {
    pub fn new(original_destination_exit_map: BidirMap<Destination, Exit>,
               original_links: BidirMap<Door, Door>) -> Shuffler {
        Shuffler {
            original_destination_exit_map,
            original_links
        }
    }

    // TODO: remove calls to .clone() by using room id
    pub fn shuffle_rooms(&self, first_room: Room, all_rooms: &[Room], rng: &mut StdRng) -> Vec<Door> {
        let mut unselected_rooms: Vec<Room> = all_rooms.iter().filter(|&room| *room != first_room).map(|room| room.clone()).collect();
        let mut exits: Vec<Exit> = first_room.one_way_exits.iter().chain(first_room.two_way_exits.iter()).map(|&exit| exit).collect();
        let mut leftover_one_way_entrances: Vec<Destination> = Vec::new();
        let mut doors: Vec<Door> = Vec::new();

        let num_iterations = unselected_rooms.len();
        for _ in 0..num_iterations {
            let (new_exits, mut new_doors, selected_room, mut entrances) = self.connect_new_room(&exits, &unselected_rooms, rng);
            exits = new_exits;
            leftover_one_way_entrances.append(&mut entrances);
            doors.append(&mut new_doors);
            let new_unselected_rooms: Vec<Room> = unselected_rooms.iter().filter(|&room| *room != selected_room).map(|room| room.clone()).collect();
            unselected_rooms = new_unselected_rooms;

            // TODO: add better logging
            //println!("=======================================");
            //println!("{:?}", doors);
            //println!("{:?}", unselected_rooms);
            //println!("=======================================");

        }

        let one_way_exits = exits.iter()
            .filter(|&exit| exit.exit_type == ExitType::OneWay)
            .map(|&exit| exit)
            .collect::<Vec<Exit>>();

        let two_way_exits = exits.iter()
            .filter(|&exit| exit.exit_type == ExitType::TwoWay)
            .map(|&exit| exit)
            .collect::<Vec<Exit>>();

        assert_eq!(one_way_exits.len(), leftover_one_way_entrances.len());
        assert!(two_way_exits.len() % 2 == 0);

        for (index, one_way_exit) in one_way_exits.iter().enumerate() {
            doors.push(Door(leftover_one_way_entrances[index], *one_way_exit));
        }

        let split_index: usize = two_way_exits.len() / 2;
        let (two_way_exits_first_half, two_way_exits_last_half) = two_way_exits.split_at(split_index);

        for (exit1, exit2) in two_way_exits_first_half.iter().zip(two_way_exits_last_half) {
            let exit1_entrance = self.find_corresponding_destination(*exit1);
            let exit2_entrance = self.find_corresponding_destination(*exit2);
            doors.push(Door(exit2_entrance, *exit1));
            doors.push(Door(exit1_entrance, *exit2));
        }

        doors
    }

    // return new list of exits, new doors to add, selected room to be removed from the pool, and the leftover entrances
    fn connect_new_room(&self, exits: &[Exit], unselected_rooms: &[Room], rng: &mut StdRng) -> (Vec<Exit>, Vec<Door>, Room, Vec<Destination>) {
        let selectable_rooms: Vec<Room> = self.find_selectable_rooms(exits, unselected_rooms);
        let selected_room = rng.choose(&selectable_rooms).expect("Could not find room");
        let (selected_exit, remaining_exits, new_doors, leftover_one_way_entrances) = self.make_room_connection(exits, selected_room, rng);
        (self.calculate_new_exits(exits, selected_exit, &remaining_exits), new_doors, selected_room.clone(), leftover_one_way_entrances)
    }

    fn calculate_new_exits(&self, exits: &[Exit], selected_exit: Exit, remaining_exits: &[Exit]) -> Vec<Exit> {
        exits.iter().filter(|&&exit| exit != selected_exit).chain(remaining_exits).map(|&exit| exit).collect::<Vec<Exit>>()
    }

    // return the exit selected, the remaining room exits (in case we picked a 2-way entrance), the door(s) linked, and the leftover entrances
    fn make_room_connection(&self, exits: &[Exit], selected_room: &Room, rng: &mut StdRng) -> (Exit, Vec<Exit>, Vec<Door>, Vec<Destination>) {
        let one_way_exit_exists = self.exit_type_exists(exits, ExitType::OneWay);
        let two_way_exit_exists =  self.exit_type_exists(exits, ExitType::TwoWay);
        let one_way_entrance_exists = !selected_room.one_way_entrances.is_empty();
        let two_way_entrance_exists = !selected_room.two_way_entrances.is_empty();

        if (one_way_exit_exists && one_way_entrance_exists) && (two_way_exit_exists && two_way_entrance_exists) {
            let exit = rng.choose(exits).expect("Could not find exit");
            match exit.exit_type {
                ExitType::OneWay => self.build_room_connection_info(selected_room, rng, *exit, true),
                ExitType::TwoWay => self.build_room_connection_info(selected_room, rng, *exit, false)
            }
        } else if one_way_exit_exists && one_way_entrance_exists {
            let one_way_exits = self.get_exits_of_type(exits, ExitType::OneWay);
            let exit = rng.choose(&one_way_exits).expect("Could not find exit");
            self.build_room_connection_info(selected_room, rng, *exit, true)
        } else if two_way_exit_exists && two_way_entrance_exists {
            let two_way_exits = self.get_exits_of_type(exits, ExitType::TwoWay);
            let exit = rng.choose(&two_way_exits).expect("Could not find exit");
            self.build_room_connection_info(selected_room, rng, *exit, false)
        } else {
            panic!("No valid entrance-exit pair found.");
        }
    }

    fn get_exits_of_type(&self, exits: &[Exit], exit_type: ExitType) -> Vec<Exit> {
        exits.iter().filter(|&exit| exit.exit_type == exit_type).map(|&exit| exit).collect::<Vec<Exit>>()
    }

    fn build_room_connection_info(&self, selected_room: &Room, rng: &mut StdRng, exit: Exit, one_way: bool) -> (Exit, Vec<Exit>, Vec<Door>, Vec<Destination>) {
        let entrance = if one_way {
            rng.choose(&selected_room.one_way_entrances).expect("Could not find one-way entrance")
        } else {
            rng.choose(&selected_room.two_way_entrances).expect("Could not find two-way entrance")
        };
        let doors = self.make_doors(*entrance, exit, one_way);
        let remaining_exits = self.find_remaining_exits(selected_room, *entrance, one_way);
        let leftover_one_way_entrances: Vec<Destination> = selected_room.one_way_entrances.iter()
            .filter(|&room_entrance| room_entrance != entrance)
            .map(|&entrance| entrance)
            .collect();
        (exit, remaining_exits, doors, leftover_one_way_entrances)
    }

    fn find_remaining_exits(&self, selected_room: &Room, entrance: Destination, one_way: bool) -> Vec<Exit> {
        if one_way {
            selected_room.one_way_exits.iter().chain(selected_room.two_way_exits.iter()).map(|&exit| exit).collect::<Vec<Exit>>()
        } else {
            let two_way_exits = selected_room.two_way_exits.iter()
                .filter(|&&exit| self.find_corresponding_exit(entrance) != exit);
            selected_room.one_way_exits.iter().chain(two_way_exits).map(|&exit| exit).collect::<Vec<Exit>>()
        }
    }

    // TODO: handle weird doors (such as the liar door in RRuins)
    fn make_doors(&self, entrance: Destination, exit: Exit, one_way: bool) -> Vec<Door> {
        if one_way {
            vec![Door(entrance, exit)]
        } else {
            vec![Door(entrance, exit), Door(self.find_corresponding_destination(exit), self.find_corresponding_exit(entrance))]
        }
    }

    fn find_selectable_rooms(&self, exits: &[Exit], unselected_rooms: &[Room]) -> Vec<Room> {
        unselected_rooms.iter()
            .filter(|&room| self.room_does_not_block_full_access(room, exits, unselected_rooms) && self.room_has_matching_entrance(room, exits))
            .map(|room| room.clone())
            .collect::<Vec<Room>>()
    }

    fn room_has_matching_entrance(&self, room: &Room, exits: &[Exit]) -> bool {
        let one_way_entrance_exists = !room.one_way_entrances.is_empty();
        // search existing exits for a one-way exit and stop searching once one is found
        let one_way_exit_exists = self.exit_type_exists(exits, ExitType::OneWay);

        let two_way_entrance_exists = !room.two_way_entrances.is_empty();
        // search existing exits for a two-way exit and stop searching once one is found
        let two_way_exit_exists = self.exit_type_exists(exits, ExitType::TwoWay);

        (one_way_entrance_exists && one_way_exit_exists) || (two_way_entrance_exists && two_way_exit_exists)
    }

    fn exit_type_exists(&self, exits: &[Exit], exit_type: ExitType) -> bool {
        exits.iter().skip_while(|&exit| exit.exit_type != exit_type).next().is_some()
    }

    fn room_does_not_block_full_access(&self, room: &Room, exits: &[Exit], unselected_rooms: &[Room]) -> bool {
        let (new_one_way_exits, new_two_way_exits) = self.count_new_exits(exits, room);

        // check if there is a room with a one way entrance and two way exit
        let room_with_opposing_connections_one_to_two_exists = self.check_opposing_exit_room_exists_one_to_two(unselected_rooms);
        // check if there is a room with a two way entrance and one way exit
        let room_with_opposing_connections_two_to_one_exists = self.check_opposing_exit_room_exists_two_to_one(unselected_rooms);

        if new_one_way_exits > 0 || new_two_way_exits > 0 {
            unselected_rooms.iter().skip_while(|&rm| self.validate_room(
                rm,
                new_one_way_exits,
                new_two_way_exits,
                room_with_opposing_connections_one_to_two_exists,
                room_with_opposing_connections_two_to_one_exists
            )).next().is_none()
        } else {
            false
        }

    }

    fn check_opposing_exit_room_exists_one_to_two(&self, unselected_rooms: &[Room]) -> bool {
        unselected_rooms.iter().skip_while(|&room| room.one_way_entrances.is_empty() || room.two_way_exits.is_empty()).next().is_some()
    }

    fn check_opposing_exit_room_exists_two_to_one(&self, unselected_rooms: &[Room]) -> bool {
        unselected_rooms.iter().skip_while(|&room| room.two_way_entrances.is_empty() || room.one_way_exits.is_empty()).next().is_some()
    }

    fn count_new_exits(&self, exits: &[Exit], room: &Room) -> (usize, usize) {
        let selected_one_way_exit_count: usize = exits.iter()
            .filter(|&exit| exit.exit_type == ExitType::OneWay)
            .collect::<Vec<&Exit>>()
            .len();
        let selected_two_way_exit_count: usize = exits.iter()
            .filter(|&exit| exit.exit_type == ExitType::TwoWay)
            .collect::<Vec<&Exit>>()
            .len();

        let has_one_way_entrance = !room.one_way_entrances.is_empty();
        let has_two_way_entrance = !room.two_way_entrances.is_empty();

        let room_one_way_exit_count = room.one_way_exits.len();
        // unless there is a one way entrance to the room, at least one exit must be removed, since it will be used as an entrance
        let room_two_way_exit_count = self.calculate_two_way_exit_count(room);

        if has_one_way_entrance && has_two_way_entrance {
            (self.decrement_count(selected_one_way_exit_count) + room_one_way_exit_count, self.decrement_count(selected_two_way_exit_count) + room_two_way_exit_count)
        } else if has_one_way_entrance {
            (self.decrement_count(selected_one_way_exit_count) + room_one_way_exit_count, selected_two_way_exit_count + room_two_way_exit_count)
        } else if has_two_way_entrance {
            (selected_one_way_exit_count + room_one_way_exit_count, self.decrement_count(selected_two_way_exit_count) + room_two_way_exit_count)
        } else {
            panic!("Room has no entrances");
        }
    }

    fn calculate_two_way_exit_count(&self, room: &Room) -> usize {
        if !room.one_way_entrances.is_empty() {
            room.two_way_exits.len()
        } else {
            self.decrement_count(room.two_way_exits.len())
        }
    }

    fn decrement_count(&self, count: usize) -> usize {
        if count > 1 { count - 1 } else { 0 }
    }

    fn validate_room(
        &self,
        room_to_validate: &Room,
        new_one_way_exits: usize,
        new_two_way_exits: usize,
        room_with_opposing_connections_one_to_two_exists: bool,
        room_with_opposing_connections_two_to_one_exists: bool
    ) -> bool {
        let has_one_way_entrance = !room_to_validate.one_way_entrances.is_empty();
        let has_two_way_entrance = !room_to_validate.two_way_entrances.is_empty();
        if has_one_way_entrance {
            (new_one_way_exits > 0) || room_with_opposing_connections_two_to_one_exists
        } else if has_two_way_entrance {
            (new_two_way_exits > 0) || room_with_opposing_connections_one_to_two_exists
        } else {
            panic!("Room has no entrances");
        }
    }

    fn find_corresponding_exit(&self, destination: Destination) -> Exit {
        let exit = *self.original_destination_exit_map.get_by_first(&destination).unwrap();
        let other_door = *self.original_links.get_by_first(&Door(destination, exit))
            .unwrap_or_else(|| self.original_links.get_by_second(&Door(destination, exit)).unwrap());
        other_door.1
    }

    fn find_corresponding_destination(&self, exit: Exit) -> Destination {
        let destination = *self.original_destination_exit_map.get_by_second(&exit).unwrap();
        let other_door = *self.original_links.get_by_first(&Door(destination, exit))
            .unwrap_or_else(|| self.original_links.get_by_second(&Door(destination, exit)).unwrap_or_else(|| panic!("{:?}", exit)));
        other_door.0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
