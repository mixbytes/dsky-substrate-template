#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use frame_support::{
    codec::{Decode, Encode},
    storage::StorageDoubleMap,
    dispatch::fmt::Debug,
    sp_runtime::sp_std::ops::{Sub, Div},
    decl_error, decl_event, decl_module, decl_storage, dispatch, ensure,    
    weights::Weight,
    Parameter,
    traits::Get,
};
use az::Cast;
use sp_std::str::FromStr;

use frame_system::ensure_signed;
use pallet_ds_accounts as accounts;
use accounts::REGISTRAR_ROLE;

mod default_weight;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub const GREEN_AREA: u8 = 0b00000001;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Point2D<Coord> {
    lon: Coord,
    lat: Coord,
}

impl<
        Coord: PartialOrd + Sub<Output = Coord>
    > Point2D<Coord> 
{
    pub fn new(lon: Coord, lat: Coord) -> Self {
        Point2D{lon, lat}
    }

    /// there is no shared trait implementing method abs(), so it's written like that
    pub fn get_distance_vector(self, second_point: Point2D<Coord>) -> Point2D<Coord> {
        let lat_length = if self.lat > second_point.lat {
            self.lat - second_point.lat
        } else {
            second_point.lat - self.lat
        };
        let long_length = if self.lon > second_point.lon {
            self.lon - second_point.lon 
        } else {
            second_point.lon - self.lon
        };
        Point2D::new(lat_length, long_length)
    }
}

//derives and if req by compiler
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Default, Debug, Clone, PartialEq, Eq)]
pub struct Rect2D<Coord> {
    north_west: Point2D<Coord>,
    south_east: Point2D<Coord>,
}

impl<Coord> Rect2D<Coord> {
    pub fn new(north_west: Point2D<Coord>, south_east: Point2D<Coord>) -> Self {
        Rect2D{north_west, south_east}
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default)]
pub struct Zone<Coord, LightCoord> {
    pub zone_id: ZoneId,
    pub rect: Rect2D<Coord>,
    pub height: LightCoord,
}

impl<Coord, LightCoord> Zone<Coord, LightCoord> {
    pub fn new(zone_id: ZoneId, rect: Rect2D<Coord>, height: LightCoord) -> Self {
        Zone {zone_id, rect, height}
    }
} 

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, Debug, PartialEq, Eq)]
pub struct Point3D<Coord> {
    lat: Coord,
    lon: Coord,
    alt: Coord,
}

impl<Coord> Point3D<Coord> {
    pub fn new(lat: Coord, lon: Coord, alt: Coord) -> Self {
        Point3D{lat, lon, alt}
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Default, Debug, Clone, PartialEq, Eq)]
pub struct Box3D<Coord> {
    pub north_west: Point3D<Coord>,
    pub south_east: Point3D<Coord>,
}

impl <Coord> Box3D<Coord> {
    pub fn new(north_west: Point3D<Coord>, south_east: Point3D<Coord>) -> Self {
        Box3D{north_west, south_east}
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Default, Debug)]
pub struct RootBox<Coord> {
    pub id: RootId,
    pub bounding_box: Box3D<Coord>,
    pub delta: Coord,
}

impl<Coord> RootBox <Coord> {
    pub fn new(id: RootId, bounding_box: Box3D<Coord>, delta: Coord) -> Self {
        RootBox{id, bounding_box, delta}
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Default, Debug, PartialEq)]
pub struct Area {
    pub area_type: u8,
    pub child_count: u16,
}

impl Area {
    pub fn new(area_type: u8, child_count: u16) -> Self {
        Area{area_type, child_count}
    } 
}

type AreaId = u16;
type RootId = u32;
type ZoneId = u64;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Trait: accounts::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    // Describe pallet constants.
    // Lean more https://substrate.dev/docs/en/knowledgebase/runtime/metadata
    type WeightInfo: WeightInfo;

    // new types, consider descriptions
    type Coord: Default 
    + Parameter
    + Copy
    + PartialOrd
    + PartialEq
    + FromStr
    + Default
    + Cast<u16>
    + Sub<Output = Self::Coord>
    + Div<Output = Self::Coord>;

    type LightCoord: Default 
    + Parameter
    + Copy
    + PartialOrd
    + FromStr;

    type MaxBuildingsInArea: Get<u16>;
    type MaxHeight: Get<Self::LightCoord>;
}    

pub trait WeightInfo {
    fn root_add() -> Weight;
    fn zone_add() -> Weight;
    fn change_area_type() -> Weight;
}

decl_storage! {
    // A unique name is used to ensure that the pallet's storage items are isolated.
    // This name may be updated, but each pallet in the runtime must use a unique name.
    // ---------------------------------vvvvvvvvvvvv
    trait Store for Module<T: Trait> as DSMapsModule {
        // MAX is 4_294_967_295. Change if required more.
        TotalRoots get(fn total_roots): RootId = 0;    

        RootBoxes get(fn root_box_data): 
            map hasher(blake2_128_concat) RootId => RootBoxOf<T>;

        AreaData get(fn area_info): 
            double_map hasher(blake2_128_concat) RootId, 
                       hasher(blake2_128_concat) AreaId => Area;    

        RedZones get(fn zone_data): 
            map hasher(blake2_128_concat) ZoneId => ZoneOf<T>;
    }
}

pub type RootBoxOf<T> = RootBox<<T as Trait>::Coord>;
pub type ZoneOf<T> = Zone<<T as Trait>::Coord, <T as Trait>::LightCoord>;

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
    {
        // Event documentation should end with an array that provides descriptive names for event parameters.
        /// New root box has been created [box number, who]
        RootCreated(RootId, AccountId),
        /// New zone added [root, area, zone number, who]
        ZoneCreated(RootId, AreaId, ZoneId, AccountId),
        /// area type changed [role, area, root, who]
        AreaTypeChanged(u8, AreaId, RootId, AccountId),
    }
);

// Errors inform users that something went wrong.
// learn more https://substrate.dev/docs/en/knowledgebase/runtime/errors
decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Error names should be descriptive.
        NoneValue,
        /// Operation is not valid
        InvalidAction,
        /// Incorrect data provided
        InvalidData,
        /// Origin do not have sufficient privileges to perform the operation
        NotAuthorized,
        /// Account doesn't exist
        NotExists,
        /// Area is unavailable for operation
        ForbiddenArea,
        /// Trying to add zone in non-existing root
        RootDoesNotExist,
        /// Sizes are off bounds
        BadDimesions,
        /// Area can't contain no more buildings
        AreaFull,
        // add additional errors below
    }
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;
        /// Adds new root to storage
        #[weight = <T as Trait>::WeightInfo::root_add()]
        pub fn root_add(origin, 
                        bounding_box: Box3D<T::Coord>,
                        delta: T::Coord) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(<accounts::Module<T>>::account_is(&who, REGISTRAR_ROLE.into()), Error::<T>::NotAuthorized);
            // TODO change ensures to inverted index (by global grid)
            let lat_dim = bounding_box.south_east.lat - bounding_box.north_west.lat;
            ensure!(lat_dim <= Self::coord_from_str("1"), Error::<T>::BadDimesions);
            let lon_dim = bounding_box.south_east.lon - bounding_box.north_west.lon;
            ensure!(lon_dim <= Self::coord_from_str("1"), Error::<T>::BadDimesions);
            ensure!(delta <= Self::coord_from_str("0.1") && 
                    delta >= Self::coord_from_str("0.002"), Error::<T>::InvalidData);

            let id = TotalRoots::get();
            let root = RootBoxOf::<T>::new(id, bounding_box, delta);
            RootBoxes::<T>::insert(id, root);
            TotalRoots::put(id + 1);
            Self::deposit_event(RawEvent::RootCreated(id, who));
            Ok(())
        }
        
        /// Form index and store input to redzones, creates area struct if it doesnt exist
        #[weight = <T as Trait>::WeightInfo::zone_add()]
        pub fn zone_add(origin, 
                        rect: Rect2D<T::Coord>,
                        height: T::LightCoord,
                        root_id: RootId) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(<accounts::Module<T>>::account_is(&who, REGISTRAR_ROLE.into()), Error::<T>::NotAuthorized);
            ensure!(RootBoxes::<T>::contains_key(root_id), Error::<T>::RootDoesNotExist);
            ensure!(height < T::MaxHeight::get(), Error::<T>::InvalidData);
            // TODO calc required area in root from rect, rn no overlap checks
            let area_id = Self::detect_intersecting_area(RootBoxes::<T>::get(root_id), rect.north_west);
            // Getting area from storage, or creating it
            let area = if AreaData::contains_key(root_id, area_id) {
                AreaData::get(root_id, area_id)
            } else {
                AreaData::insert(root_id, area_id, Area::new(GREEN_AREA, 0));
                Area::new(GREEN_AREA, 0)
            };
            ensure!(area.child_count < T::MaxBuildingsInArea::get(), Error::<T>::AreaFull);
            ensure!(area.area_type == GREEN_AREA, Error::<T>::ForbiddenArea); 

            let id = Self::pack_index(root_id, area_id, area.child_count); 
            let zone = ZoneOf::<T>::new(id, rect, height);
            RedZones::<T>::insert(id, zone);
            AreaData::mutate(root_id, area_id, |ar| {
                ar.child_count += 1;
            });
            Self::deposit_event(RawEvent::ZoneCreated(root_id, area_id, id, who));
            Ok(())
        }

        /// Changes area type with u8 bit flag
        #[weight = <T as Trait>::WeightInfo::change_area_type()]
        pub fn change_area_type(origin, 
                                root_id: RootId, 
                                area_id: AreaId, 
                                area_type: u8) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(<accounts::Module<T>>::account_is(&who, REGISTRAR_ROLE.into()), Error::<T>::NotAuthorized);
            ensure!(AreaData::contains_key(root_id, area_id), Error::<T>::NotExists);
            AreaData::mutate(root_id, area_id, |ar| {
                ar.area_type = area_type;
            });
            Self::deposit_event(RawEvent::AreaTypeChanged(area_type, area_id, root_id, who));
            Ok(())
        }
    }
}

// Module allows  use  common functionality by dispatchables
impl<T: Trait> Module<T> {
    // Implement module function.
    // Public functions can be called from other runtime modules.
    /// Returns id of an area in root, in which supplied point is located
    fn detect_intersecting_area(root_box: RootBoxOf<T>, touch: Point2D<T::Coord>) -> AreaId {
        let root_base_point: Point2D<T::Coord> = 
        Point2D::new(root_box.bounding_box.north_west.lat,
                     root_box.bounding_box.north_west.lon); 
        let root_secondary_point: Point2D<T::Coord> = 
        Point2D::new(root_box.bounding_box.south_east.lat,
                    root_box.bounding_box.south_east.lon); 
        let root_dimensions = root_base_point.get_distance_vector(root_secondary_point);
        let distance_vector = root_base_point.get_distance_vector(touch);
        
        let delta = root_box.delta;
        let touch_lon = distance_vector.lon;
        let touch_lat = distance_vector.lat;
        let root_lat_dimension = root_dimensions.lat;

        let row: u16 = (touch_lat / delta).cast() + 1;
        let column: u16 = (touch_lon / delta).cast() + 1;
        let total_rows: u16 = (root_lat_dimension / delta).cast();

        (total_rows * (column - 1)) + row
    }

    /// creates type from str, no error handling
    fn coord_from_str<Coord> (s: &str) -> Coord
            where Coord: FromStr + Default {
        match Coord::from_str(s) {
            Ok(v) => v,
            Err(_) => Default::default(),
        }
    } 

    /// form index for storing zones, wrapped in u64
    // v.............root id here............v v.....area id.....v v..child objects..v
    // 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000
    fn pack_index(root: RootId, area: AreaId, children: u16) -> ZoneId {
        (root as u64) << 32 |
        (area as u64) << 16 | 
        children as u64
    }

    #[allow(dead_code)]
    fn unpack_index(index: ZoneId) -> (RootId, AreaId, u16) {
        let mask_u16: u64 = 0x0000_0000_0000_0000_0000_0000_ffff_ffff;
        let root: RootId = (index >> 32) as u32;
        let area: AreaId = ((index >> 16) & mask_u16) as u16;
        let children: u16 = (index & mask_u16) as u16;
        
        (root, area, children)
    }
}

