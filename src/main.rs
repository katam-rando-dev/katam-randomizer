extern crate rand;
extern crate bidir_map;
extern crate csv;
#[macro_use]
extern crate serde_derive;
extern crate serde;

mod shuffler;
mod csv_loader;
mod rom;

use rand::{StdRng, SeedableRng};
use bidir_map::BidirMap;
use std::fs::File;
use shuffler::{Door, Destination, Exit, ExitType, Shuffler, Room};

fn main() {

    // TODO: ensure that the user provides the ROM!
    let file = File::open("Kirby & The Amazing Mirror (U).gba").unwrap();
    let mut game_rom = rom::Rom::new(file);

    let loader = csv_loader::CsvLoader;
    let door_table = loader.load_entrances("doordata.csv");
    let rooms = loader.load_rooms("roomdata.csv", &door_table);

    let mut original_destination_exit_map: BidirMap<Destination, Exit> = BidirMap::new();
    let mut original_links: BidirMap<Door, Door> = BidirMap::new();
    for option_record in &door_table {
        if let Some(ref record) = *option_record {
            let destination = record.extract_destination();
            let exit = record.extract_exit();
            original_destination_exit_map.insert(destination, exit);

            match exit.exit_type {
                ExitType::TwoWay => {
                    let option_linked_record = &door_table[exit.linked_door_id as usize];
                    if let Some(ref linked_record) = *option_linked_record {
                        let linked_destination = linked_record.extract_destination();
                        let linked_exit = linked_record.extract_exit();
                        original_links.insert(Door(destination, exit), Door(linked_destination, linked_exit));
                    }
                }
                _ => ()
            }
        }
    }

    let mut rng: StdRng = StdRng::from_seed(&[1usize; 32]);
    let first_room: Room = Room {
        id: 0,
        one_way_entrances: Vec::new(),
        two_way_entrances: Vec::new(),
        one_way_exits: vec![Exit::new(0, 0x873450, 0x930E04, ExitType::OneWay, -1)],
        two_way_exits: Vec::new()
    };
    let shuffler = Shuffler::new(original_destination_exit_map, original_links);
    let result = shuffler.shuffle_rooms(first_room, &rooms, &mut rng);
    println!("{}", result.len());

    for door in &result {
        let &Door(destination, exit) = door;

        let exit_addr1 = exit.exit_addr1;
        let exit_addr2 = exit.exit_addr2;
        println!("{:x} | {:x}", exit_addr1, exit_addr2);

        let destination_data = destination.destination_bytes;
        println!("{:x}, {:x}, {:x}, {:x}", destination_data[0], destination_data[1], destination_data[2], destination_data[3]);

        println!("");
        game_rom.write_bytes(&destination_data[..], exit_addr1);
        game_rom.write_bytes(&destination_data[..], exit_addr2);
    }

    game_rom.create_randomized_rom();
}