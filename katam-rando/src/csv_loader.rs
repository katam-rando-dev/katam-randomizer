use csv;
use std::u8;
use std::usize;
use std::path::Path;
use super::shuffler::{Room, Destination, Exit, ExitType};

pub type DoorTable = Vec<Option<DoorRecord>>;

#[derive(Clone, Debug, Deserialize)]
pub struct DoorRecord {
    pub doorid: usize,
    pub destination: String,
    pub exitaddr1: String,
    pub exitaddr2: String,
    pub isoneway: bool,
    pub linkeddoor: Option<i32>
}

impl DoorRecord {
    pub fn extract_destination(&self) -> Destination {
        let bytes: Vec<u8> = self.destination
            .split_whitespace()
            .map(|byte| u8::from_str_radix(byte, 16).unwrap())
            .collect();
        Destination::new(self.doorid, [bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    pub fn extract_exit(&self) -> Exit {
        let exit_addr_1 = usize::from_str_radix(self.exitaddr1.as_str(), 16).unwrap();
        let exit_addr_2 = usize::from_str_radix(self.exitaddr2.as_str(), 16).unwrap();
        let exit_type = if self.isoneway {
            ExitType::OneWay
        } else {
            ExitType::TwoWay
        };
        let linked_door = match self.linkeddoor {
            Some(doorid) => doorid,
            None => -1
        };
        Exit::new(self.doorid, exit_addr_1, exit_addr_2, exit_type, linked_door)
    }
}

#[derive(Debug, Deserialize)]
struct RoomRecord {
    roomid: usize,
    onewayentranceids: Option<String>,
    twowayentranceids: Option<String>,
    onewayexitids: Option<String>,
    twowayexitids: Option<String>
}

pub struct CsvLoader;

impl CsvLoader {
    pub fn load_entrances<P: AsRef<Path>>(&self, path: P) -> DoorTable {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(path).unwrap();
        let records: Vec<DoorRecord> = reader.deserialize()
            .map(|result| result.unwrap())
            .collect();
        let max_id_record = records.iter().max_by(|record1, record2| record1.doorid.cmp(&record2.doorid)).unwrap();
        let mut door_records = vec![None; max_id_record.doorid+1];
        records.iter().for_each(|result| door_records[result.doorid] = Some(result.clone()));
        door_records
    }

    pub fn load_rooms<P: AsRef<Path>>(&self, path: P, door_table: &DoorTable) -> Vec<Room> {
        let mut rooms = vec![];
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(path).unwrap();
        for result in reader.deserialize() {
            let record: RoomRecord = result.unwrap();

            let one_way_entrance_ids: Vec<usize> = match record.onewayentranceids {
                Some(ids) => ids.split_whitespace().map(|str_id| str_id.parse::<usize>().unwrap()).collect(),
                None => vec![]
            };

            let two_way_entrance_ids: Vec<usize> = match record.twowayentranceids {
                Some(ids) => ids.split_whitespace().map(|str_id| str_id.parse::<usize>().unwrap()).collect(),
                None => vec![]
            };

            let one_way_exit_ids: Vec<usize> = match record.onewayexitids {
                Some(ids) => ids.split_whitespace().map(|str_id| str_id.parse::<usize>().unwrap()).collect(),
                None => vec![]
            };

            let two_way_exit_ids: Vec<usize> = match record.twowayexitids {
                Some(ids) => ids.split_whitespace().map(|str_id| str_id.parse::<usize>().unwrap()).collect(),
                None => vec![]
            };

            let one_way_entrances: Vec<Destination> = one_way_entrance_ids.iter().map(|&id| door_table[id].clone().unwrap().extract_destination()).collect();
            let two_way_entrances: Vec<Destination> = two_way_entrance_ids.iter().map(|&id| door_table[id].clone().unwrap().extract_destination()).collect();
            let one_way_exits: Vec<Exit> = one_way_exit_ids.iter().map(|&id| door_table[id].clone().unwrap().extract_exit()).collect();
            let two_way_exits: Vec<Exit> = two_way_exit_ids.iter().map(|&id| door_table[id].clone().unwrap().extract_exit()).collect();

            rooms.push( Room {
                id: record.roomid,
                one_way_entrances: one_way_entrances,
                two_way_entrances: two_way_entrances,
                one_way_exits: one_way_exits,
                two_way_exits: two_way_exits
            });
        }
        rooms
    }
}