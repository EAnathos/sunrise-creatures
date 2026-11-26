//! A creature's packs and its screen (SPECS §6.5): the storage a creature carries (the vulture's chests, the elephant's),
//! saved on it (`creature:pack`: each slot's item as its NBT, read back with a host), and the screen its owner opens.
//! Users: the Vulture, the Elephant.
//!
//! A screen is a chest menu a mod opens (`open_menu`), in pages a mod describes (`Screens`): each page's rows, its
//! buttons (items in given slots, which the mod answers, `Screens::button`) and the storage slots it shows (from a
//! menu slot on, each showing a storage index). Such a menu moves nothing by itself: its clicks (`clicked`, from the
//! library's `menu_clicked`) are the screen's, each of vanilla's as a chest's slots take it: a click, a shift-click, a
//! number key or the off hand's, Q, a double-click, a drag, a creative middle-click, a click outside the window. The
//! storage's and the inventory's stacks are written directly (`Grid`: `set_contents`, `set_inventory_slot`,
//! `menu_set_items`), the cursor's through its place (`MenuRef::cursor`, in a transaction). A client only learns of a
//! cursor it changed itself: after a throw from the cursor the screen opens again in place, which puts what is left on
//! the cursor back in the inventory (sunrise-org/sunrise#3815).
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Hand, Holder, HostTransaction, ItemResource, ItemStack,
    LevelRef, MenuClickedEvent, MenuClosedEvent, MenuRef, MenuReplace, PlayerRef, Registrar, ServerHost, Tag, Text,
};

use crate::sync::Locked;

const ROW: usize = 9;
/// The player's inventory after the menu's own slots: 27 slots and the hotbar's 9.
const INVENTORY: usize = 36;
/// Vanilla's click types (`ClickType`): a click and a shift-click; the slot number of a click outside the window.
const PICKUP: i32 = 0;
const QUICK_MOVE: i32 = 1;
const OUTSIDE: i32 = -999;

/// A page of a creature's screen: its rows (a chest menu's: 1 to 6), its title, its buttons (an item in a menu slot,
/// none for an empty one), and its storage slots: from menu slot `first` on, each showing a storage index. The other
/// slots are panes.
pub struct Page {
    pub rows: u8,
    pub title: Text,
    pub buttons: Vec<(usize, Option<ItemStack>)>,
    pub first: usize,
    pub storage: Vec<usize>,
}

/// A kind's screen, as its mod describes it.
pub trait Screens: Sync {
    /// Its storage's size (slots), all its packs together.
    fn size(&self) -> usize;
    /// Page `page` of creature `id`'s screen as it is now, if it can show.
    fn page(&self, host: &dyn ServerHost, id: u32, page: u8) -> Option<Page>;
    /// Button `slot` of page `page` clicked (a shift-click when `shift`) by `player`: the page to show after, if any
    /// (none closes the screen).
    fn button(
        &self,
        host: &mut dyn ServerHost,
        player: PlayerRef,
        id: u32,
        page: u8,
        slot: usize,
        shift: bool,
    ) -> Option<u8>;
}

/// Each creature's storage, each slot's item's NBT.
static PACK: AttachmentType<Vec<Option<Compound>>> = AttachmentType {
    namespace: "creature",
    name: "pack",
    default: Vec::new,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// From the library's `register`: the storage's attachment.
pub(crate) fn declare(r: &mut dyn Registrar) {
    let _ = r.register_attachment_type("creature:pack", false);
}

// `&Vec`: the attachment's codec takes `fn(&T)` with `T` its type, a `Vec`.
#[allow(clippy::ptr_arg)]
fn encode(items: &Vec<Option<Compound>>) -> Result<Compound, AttachmentError> {
    let list = items
        .iter()
        .enumerate()
        .filter_map(|(k, s)| {
            let mut c = s.clone()?;
            c.insert("Slot".into(), Tag::Int(k as i32));
            Some(Tag::Compound(c))
        })
        .collect();
    Ok([("Items".into(), Tag::List(list))].into())
}

fn decode(data: &Compound) -> Result<Vec<Option<Compound>>, AttachmentError> {
    let mut items: Vec<Option<Compound>> = Vec::new();
    if let Some(Tag::List(list)) = data.get(&"Items".into()) {
        for tag in list {
            let Tag::Compound(c) = tag else { continue };
            let slot = match c.get(&"Slot".into()) {
                Some(Tag::Int(s)) => *s as usize,
                Some(Tag::Byte(s)) => *s as usize,
                _ => continue,
            };
            if items.len() <= slot {
                items.resize(slot + 1, None);
            }
            let mut item = c.clone();
            item.remove(&"Slot".into());
            items[slot] = Some(item);
        }
    }
    Ok(items)
}

/// Creature `id`'s storage as stacks, `size` slots.
pub fn contents(host: &mut dyn ServerHost, id: u32, size: usize) -> Vec<Option<ItemStack>> {
    let mut items: Vec<Option<ItemStack>> = PACK
        .get(host, Holder::Entity(id))
        .unwrap_or_default()
        .iter()
        .map(|c| c.as_ref().and_then(|c| ItemStack::from_nbt(&*host, c)))
        .collect();
    items.resize(size, None);
    items
}

/// Creature `id`'s storage written.
pub fn set_contents(host: &mut dyn ServerHost, id: u32, items: &[Option<ItemStack>]) {
    let nbt: Vec<Option<Compound>> = items
        .iter()
        .map(|s| s.as_ref().and_then(|s| s.to_nbt(&*host)))
        .collect();
    let _ = PACK.set(host, Holder::Entity(id), nbt);
}

/// The stacks in `range` of creature `id`'s storage, taken out (a pack taken off).
pub fn take(host: &mut dyn ServerHost, id: u32, size: usize, range: std::ops::Range<usize>) -> Vec<ItemStack> {
    let mut items = contents(host, id, size);
    let taken = items[range].iter_mut().filter_map(Option::take).collect();
    set_contents(host, id, &items);
    taken
}

/// Whether creature `id` has nothing stored (for a mod moving an older save's storage here).
pub fn is_empty(host: &mut dyn ServerHost, id: u32) -> bool {
    PACK.get(host, Holder::Entity(id))
        .unwrap_or_default()
        .iter()
        .all(Option::is_none)
}

/// Its death: its screen closes for whoever has it open, and all it carried falls at `at`.
pub fn died(host: &mut dyn ServerHost, id: u32, size: usize, level: LevelRef, at: [f64; 3]) {
    close(host, id);
    for s in take(host, id, size, 0..size) {
        crate::hand::drop_stack(host, level, at, s);
    }
}

/// Creature `id`'s screen closed for whoever has it open.
pub fn close(host: &mut dyn ServerHost, id: u32) {
    let viewers: Vec<u128> = OPEN
        .locked()
        .iter()
        .filter(|(_, o)| o.creature == id)
        .map(|(uuid, _)| *uuid)
        .collect();
    for uuid in viewers {
        host.close_menu(uuid);
    }
}

/// The screen each player has open: its creature, its pages, the one shown, the menu's window, and the page's rows and
/// storage slots.
#[derive(Clone)]
struct Open {
    creature: u32,
    screens: &'static dyn Screens,
    page: u8,
    window: i32,
    rows: u8,
    first: usize,
    storage: Vec<usize>,
}

static OPEN: Mutex<BTreeMap<u128, Open>> = Mutex::new(BTreeMap::new());

/// The menu's slots for `page`: its buttons, its storage slots, panes in the rest.
fn slots(host: &dyn ServerHost, page: &Page, storage: &[Option<ItemStack>]) -> Vec<Option<ItemStack>> {
    let pane = crate::hand::stack(host, "minecraft:gray_stained_glass_pane", 1);
    let mut items = vec![pane; usize::from(page.rows) * ROW];
    for (k, index) in page.storage.iter().enumerate() {
        if let Some(cell) = items.get_mut(page.first + k) {
            *cell = storage.get(*index).cloned().flatten();
        }
    }
    for (slot, item) in &page.buttons {
        if let Some(cell) = items.get_mut(*slot) {
            *cell = item.clone();
        }
    }
    items
}

/// Opens page `page` of creature `id`'s screen for player `uuid` (in place of what they have open, or after closing
/// it). Whether it opened.
pub fn open(
    host: &mut dyn ServerHost,
    uuid: u128,
    id: u32,
    screens: &'static dyn Screens,
    page: u8,
    replace: MenuReplace,
) -> bool {
    let Some(p) = screens.page(&*host, id, page) else {
        return false;
    };
    let storage = contents(host, id, screens.size());
    let items = slots(&*host, &p, &storage);
    let Ok(Some(window)) = host.open_menu(uuid, p.rows, p.title.clone(), items, replace) else {
        return false;
    };
    OPEN.locked().insert(
        uuid,
        Open {
            creature: id,
            screens,
            page,
            window,
            rows: p.rows,
            first: p.first,
            storage: p.storage,
        },
    );
    true
}

/// From the library's `menu_closed` hook.
pub(crate) fn closed(event: &MenuClosedEvent) {
    let mut open = OPEN.locked();
    if open.get(&event.uuid).is_some_and(|o| o.window == event.window) {
        open.remove(&event.uuid);
    }
}

/// From the library's `menu_clicked` hook: a button, or a click on the storage's or the inventory's slots, as a
/// chest's (vanilla's `AbstractContainerMenu.doClick`): a click picks a stack up, puts the cursor's down or swaps them
/// (a right-click picks up half, puts down one); a shift-click sends a stack across; a number key (or the off hand's)
/// swaps a slot with that hotbar slot; Q throws one of a slot's stack (all of it with control); a double-click gathers
/// the same item onto the cursor; a drag spreads the cursor's stack over the slots crossed (evenly, or one each with
/// the right button; a full stack each with the middle one in creative, as a middle-click picks a full stack up); a
/// click outside the window throws the cursor's stack (one with a right-click).
pub(crate) fn clicked(host: &mut dyn ServerHost, event: &MenuClickedEvent) {
    let Some(open) = OPEN.locked().get(&event.uuid).cloned() else {
        return;
    };
    if open.window != event.window {
        return;
    }
    let Some(menu) = host.menu(event.uuid, event.window) else {
        return;
    };
    let Some(player) = host
        .players()
        .server_players()
        .into_iter()
        .find(|p| p.uuid() == event.uuid)
    else {
        return;
    };
    // Its storage went with it (`died`): its screen closes, and nothing goes into it.
    let alive = host
        .entity(open.creature)
        .and_then(|e| host.living(e))
        .and_then(|l| l.alive(&*host))
        .unwrap_or(false);
    if !alive {
        host.close_menu(event.uuid);
        return;
    }
    let right = event.button == 1;
    if event.mode == QUICK_CRAFT {
        drag(host, player, menu, &open, event);
        return;
    }
    if event.slot == OUTSIDE {
        if event.mode == PICKUP && drop_cursor(host, player, menu, right) {
            // The client isn't told of a cursor it didn't change itself: the screen opens again in place
            // (sunrise-org/sunrise#3815).
            open_again(host, event.uuid, &open, open.page);
        }
        return;
    }
    let Ok(slot) = usize::try_from(event.slot) else {
        return;
    };
    let Some(cell) = cell(&open, slot) else {
        // A button (any of the page's own slots that isn't storage).
        if slot < usize::from(open.rows) * ROW && matches!(event.mode, PICKUP | QUICK_MOVE) {
            let shift = event.mode == QUICK_MOVE;
            match open.screens.button(host, player, open.creature, open.page, slot, shift) {
                Some(page) => open_again(host, event.uuid, &open, page),
                None => host.close_menu(event.uuid),
            }
        }
        return;
    };
    let mut grid = Grid::read(host, player, &open);
    let cursor = menu.cursor().stack(host);
    let creative = player.has_infinite_materials(&*host).unwrap_or(false);
    let cursor_now = match event.mode {
        PICKUP => {
            let (cursor_now, slot_now) = pick(&*host, cursor.clone(), grid.at(cell).take(), right);
            *grid.at(cell) = slot_now;
            cursor_now
        }
        QUICK_MOVE => {
            quick_move(&*host, &mut grid, &open, cell);
            cursor.clone()
        }
        SWAP => {
            swap(host, player, &mut grid, cell, event.button);
            cursor.clone()
        }
        THROW if cursor.is_none() => {
            throw(host, player, &mut grid, cell, event.button == 1);
            None
        }
        CLONE if creative && cursor.is_none() => grid.at(cell).clone().map(|s| full(&*host, s)),
        PICKUP_ALL => cursor.clone().map(|c| gather(&*host, &mut grid, &open, c)),
        _ => return,
    };
    if !same(&cursor, &cursor_now) && !set_cursor(host, menu, cursor.as_ref(), cursor_now.as_ref()) {
        return;
    }
    grid.write(host, player, &open);
}

/// The screen shown again in place, on `page` (closed if that page can't show).
fn open_again(host: &mut dyn ServerHost, uuid: u128, open: &Open, page: u8) {
    if !self::open(host, uuid, open.creature, open.screens, page, MenuReplace::InPlace) {
        host.close_menu(uuid);
    }
}

/// Vanilla's click types (`ClickType`) after a click and a shift-click, and the off hand's button for a swap (F).
const SWAP: i32 = 2;
const CLONE: i32 = 3;
const THROW: i32 = 4;
const QUICK_CRAFT: i32 = 5;
const PICKUP_ALL: i32 = 6;
const OFF_HAND: i32 = 40;

/// A slot of the screen that holds items: one of the creature's storage (its index there), or the inventory's (0 to 8
/// the hotbar).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cell {
    Chest(usize),
    Inventory(usize),
}

/// The cell behind menu slot `slot`: the page's storage slots, then, after its rows, the inventory's 27 slots and the
/// hotbar.
fn cell(open: &Open, slot: usize) -> Option<Cell> {
    if let Some(index) = slot.checked_sub(open.first).and_then(|k| open.storage.get(k)) {
        return Some(Cell::Chest(*index));
    }
    let k = slot
        .checked_sub(usize::from(open.rows) * ROW)
        .filter(|&k| k < INVENTORY)?;
    Some(Cell::Inventory(if k < 27 { 9 + k } else { k - 27 }))
}

/// Every cell, in the menu's order (the page's storage, then the inventory as shown).
fn cells(open: &Open) -> Vec<Cell> {
    let chests = open.storage.iter().map(|&k| Cell::Chest(k));
    chests.chain((9..INVENTORY).chain(0..9).map(Cell::Inventory)).collect()
}

/// The chests' and the inventory's stacks while a click changes them.
struct Grid {
    chests: Vec<Option<ItemStack>>,
    inventory: Vec<Option<ItemStack>>,
    before: Vec<Option<ItemStack>>,
}

impl Grid {
    fn read(host: &mut dyn ServerHost, player: PlayerRef, open: &Open) -> Grid {
        let chests = contents(host, open.creature, open.screens.size());
        let mut inventory: Vec<Option<ItemStack>> = vec![None; INVENTORY];
        for (k, s) in player.inventory_slots(&*host).unwrap_or_default() {
            if let Some(cell) = inventory.get_mut(k as usize) {
                *cell = Some(s);
            }
        }
        let before = inventory.clone();
        Grid {
            chests,
            inventory,
            before,
        }
    }

    fn at(&mut self, cell: Cell) -> &mut Option<ItemStack> {
        match cell {
            Cell::Chest(k) => &mut self.chests[k],
            Cell::Inventory(k) => &mut self.inventory[k],
        }
    }

    /// The inventory's changed slots written to the player, the storage saved, the menu's slots sent again.
    fn write(&self, host: &mut dyn ServerHost, player: PlayerRef, open: &Open) {
        for (k, (now, was)) in self.inventory.iter().zip(&self.before).enumerate() {
            if !same(now, was) {
                let _ = player.set_inventory_slot(host, k as u32, now.clone());
            }
        }
        set_contents(host, open.creature, &self.chests);
        if let Some(page) = open.screens.page(&*host, open.creature, open.page) {
            let _ = host.menu_set_items(player.uuid(), open.window, slots(&*host, &page, &self.chests));
        }
    }
}

/// The same item and count (or both empty).
fn same(a: &Option<ItemStack>, b: &Option<ItemStack>) -> bool {
    a.as_ref().map(|s| (s.item(), s.count())) == b.as_ref().map(|s| (s.item(), s.count()))
}

/// `stack`, `n` of it (none for 0).
fn with(stack: &ItemStack, n: u32) -> Option<ItemStack> {
    let mut s = stack.clone();
    (n > 0 && s.set_count(n).is_ok()).then_some(s)
}

fn max_of(host: &dyn ServerHost, stack: &ItemStack) -> u32 {
    host.max_stack_size(stack.item()).unwrap_or(64)
}

/// A full stack of `stack`'s item.
fn full(host: &dyn ServerHost, stack: ItemStack) -> ItemStack {
    let max = max_of(host, &stack);
    with(&stack, max).unwrap_or(stack)
}

/// A shift-click: the stack sent across, from a chest to the inventory or back, as much as fits.
fn quick_move(host: &dyn ServerHost, grid: &mut Grid, open: &Open, cell: Cell) {
    let Some(stack) = grid.at(cell).take() else {
        return;
    };
    let rest = match cell {
        // Vanilla's order for a stack leaving a chest: the hotbar from its end, then the inventory from its end.
        Cell::Chest(_) => {
            let order: Vec<usize> = (0..9).rev().chain((9..INVENTORY).rev()).collect();
            transfer(host, stack, &mut grid.inventory, &order)
        }
        Cell::Inventory(_) => {
            let order = open.storage.clone();
            transfer(host, stack, &mut grid.chests, &order)
        }
    };
    *grid.at(cell) = rest;
}

/// A number key (`button` 0 to 8) or the off hand's key over `cell`: the two stacks swap.
fn swap(host: &mut dyn ServerHost, player: PlayerRef, grid: &mut Grid, cell: Cell, button: i32) {
    if button == OFF_HAND {
        let off = crate::hand::held(&*host, player, Hand::Off);
        let here = grid.at(cell).take();
        if crate::hand::set_held(host, player, Hand::Off, here.clone()) {
            *grid.at(cell) = off;
        } else {
            *grid.at(cell) = here;
        }
        return;
    }
    let Some(k) = usize::try_from(button).ok().filter(|&k| k < 9) else {
        return;
    };
    let hotbar = Cell::Inventory(k);
    if hotbar != cell {
        let a = grid.at(cell).take();
        let b = std::mem::replace(grid.at(hotbar), a);
        *grid.at(cell) = b;
    }
}

/// Q over `cell`: one of its stack thrown, or all of it.
fn throw(host: &mut dyn ServerHost, player: PlayerRef, grid: &mut Grid, cell: Cell, all: bool) {
    let Some(stack) = grid.at(cell).take() else {
        return;
    };
    let n = if all { stack.count() } else { 1 };
    *grid.at(cell) = with(&stack, stack.count() - n);
    if let Some(thrown) = with(&stack, n) {
        let _ = player.drop_stack(host, thrown, true);
    }
}

/// A double-click with `cursor` held: the same item gathered onto it from every slot, the slots that aren't full first,
/// up to a full stack. What the cursor holds after.
fn gather(host: &dyn ServerHost, grid: &mut Grid, open: &Open, cursor: ItemStack) -> ItemStack {
    let max = max_of(host, &cursor);
    let mut count = cursor.count();
    for full_too in [false, true] {
        for cell in cells(open) {
            if count >= max {
                break;
            }
            let Some(s) = grid.at(cell).clone().filter(|s| s.is_stackable_with(&cursor)) else {
                continue;
            };
            if !full_too && s.count() >= max {
                continue;
            }
            let n = s.count().min(max - count);
            count += n;
            *grid.at(cell) = with(&s, s.count() - n);
        }
    }
    with(&cursor, count).unwrap_or(cursor)
}

/// A drag in progress per player: its kind (0 evenly, 1 one each, 2 a full stack each) and the slots it crossed.
static DRAGS: Mutex<BTreeMap<u128, (i32, Vec<usize>)>> = Mutex::new(BTreeMap::new());

/// A drag (`QUICK_CRAFT`): it starts (the button's stage 0), crosses slots (1) and ends (2), when the cursor's stack is
/// spread over the slots crossed that are empty or hold the same item, as vanilla's `getQuickCraftPlaceCount`: evenly,
/// one each, or a full stack each (creative only); what is left stays on the cursor. A drag over one slot is a click.
fn drag(host: &mut dyn ServerHost, player: PlayerRef, menu: MenuRef, open: &Open, event: &MenuClickedEvent) {
    let (stage, kind) = (event.button & 3, (event.button >> 2) & 3);
    let mut drags = DRAGS.locked();
    match stage {
        0 => {
            drags.insert(event.uuid, (kind, Vec::new()));
        }
        1 => {
            if let (Some((k, slots)), Ok(slot)) = (drags.get_mut(&event.uuid), usize::try_from(event.slot))
                && *k == kind
                && cell(open, slot).is_some()
                && !slots.contains(&slot)
            {
                slots.push(slot);
            }
        }
        _ => {
            let Some((k, slots)) = drags.remove(&event.uuid).filter(|d| d.0 == kind) else {
                return;
            };
            drop(drags);
            spread(host, player, menu, open, k, &slots);
        }
    }
}

/// The end of a drag of kind `kind` over `slots`.
fn spread(host: &mut dyn ServerHost, player: PlayerRef, menu: MenuRef, open: &Open, kind: i32, slots: &[usize]) {
    let Some(cursor) = menu.cursor().stack(host) else {
        return;
    };
    let creative = player.has_infinite_materials(&*host).unwrap_or(false);
    if kind == 2 && !creative {
        return;
    }
    let mut grid = Grid::read(host, player, open);
    let mut cells: Vec<Cell> = slots
        .iter()
        .filter_map(|&s| cell(open, s))
        .filter(|&c| grid.at(c).as_ref().is_none_or(|s| s.is_stackable_with(&cursor)))
        .collect();
    if kind == 0 {
        cells.truncate(cursor.count() as usize);
    }
    let cursor_now = if let [one] = cells[..] {
        let (cursor_now, slot_now) = pick(&*host, Some(cursor.clone()), grid.at(one).take(), kind == 1);
        *grid.at(one) = slot_now;
        cursor_now
    } else if cells.is_empty() {
        return;
    } else {
        let max = max_of(&*host, &cursor);
        let each = match kind {
            0 => cursor.count() / cells.len() as u32,
            1 => 1,
            _ => max,
        };
        let mut left = cursor.count();
        for c in cells {
            let here = grid.at(c).as_ref().map_or(0, |s| s.count());
            let now = (here + each).min(max);
            if kind != 2 {
                left -= now - here;
            }
            *grid.at(c) = with(&cursor, now);
        }
        with(&cursor, left)
    };
    if !same(&Some(cursor.clone()), &cursor_now) && !set_cursor(host, menu, Some(&cursor), cursor_now.as_ref()) {
        return;
    }
    grid.write(host, player, open);
}

/// A click (`right` or left) on a slot holding `slot`, with `cursor` on the cursor, as vanilla's
/// `AbstractContainerMenu.doClick` for a plain slot: what the cursor and the slot hold after it.
fn pick(
    host: &dyn ServerHost,
    cursor: Option<ItemStack>,
    slot: Option<ItemStack>,
    right: bool,
) -> (Option<ItemStack>, Option<ItemStack>) {
    match (cursor, slot) {
        (None, None) => (None, None),
        // Picking up: all of it, or half (the larger half) with a right-click.
        (None, Some(s)) => {
            let n = if right { s.count().div_ceil(2) } else { s.count() };
            (with(&s, n), with(&s, s.count() - n))
        }
        // Putting down: all of it, or one with a right-click.
        (Some(c), None) => {
            let max = host.max_stack_size(c.item()).unwrap_or(64);
            let n = if right { 1 } else { c.count().min(max) };
            (with(&c, c.count() - n), with(&c, n))
        }
        (Some(c), Some(s)) if s.is_stackable_with(&c) => {
            let max = host.max_stack_size(c.item()).unwrap_or(64);
            let room = max.saturating_sub(s.count());
            let n = if right { room.min(1) } else { room.min(c.count()) };
            (with(&c, c.count() - n), with(&s, s.count() + n))
        }
        // Another item: they swap.
        (Some(c), Some(s)) => (Some(s), Some(c)),
    }
}

/// Puts `now` on `menu`'s cursor in place of `was`, in one transaction: whether it took.
fn set_cursor(host: &mut dyn ServerHost, menu: MenuRef, was: Option<&ItemStack>, now: Option<&ItemStack>) -> bool {
    host.transaction(|host, tx| {
        let cursor = menu.cursor();
        if let Some(old) = was
            && cursor.extract(host, tx, &ItemResource::of(old), old.count()) != old.count()
        {
            return Err(());
        }
        if let Some(new) = now
            && cursor.insert(host, tx, &ItemResource::of(new), new.count()) != new.count()
        {
            return Err(());
        }
        Ok(())
    })
    .is_ok()
}

/// A click outside the window: the cursor's stack is thrown, or one of it with a right-click. Whether it was.
fn drop_cursor(host: &mut dyn ServerHost, player: PlayerRef, menu: MenuRef, right: bool) -> bool {
    let Some(stack) = menu.cursor().stack(host) else {
        return false;
    };
    let n = if right { 1 } else { stack.count() };
    let mut thrown = stack.clone();
    if thrown.set_count(n).is_err() {
        return false;
    }
    let mut left = stack.clone();
    let rest = (stack.count() > n && left.set_count(stack.count() - n).is_ok()).then_some(left);
    let dropped = set_cursor(host, menu, Some(&stack), rest.as_ref());
    if dropped {
        let _ = player.drop_stack(host, thrown, true);
    }
    dropped
}

/// Puts `stack` into `to`'s slots in `order`, those holding the same item first, then empty ones: what doesn't fit.
fn transfer(
    host: &dyn ServerHost,
    stack: ItemStack,
    to: &mut [Option<ItemStack>],
    order: &[usize],
) -> Option<ItemStack> {
    let max = host.max_stack_size(stack.item()).unwrap_or(64);
    let mut left = stack.count();
    for &k in order {
        if let Some(s) = to[k].as_mut().filter(|s| s.is_stackable_with(&stack)) {
            let n = left.min(max.saturating_sub(s.count()));
            if n > 0 && s.set_count(s.count() + n).is_ok() {
                left -= n;
            }
        }
    }
    for &k in order {
        if left == 0 {
            break;
        }
        if to[k].is_none() {
            let mut s = stack.clone();
            let n = left.min(max);
            if s.set_count(n).is_ok() {
                to[k] = Some(s);
                left -= n;
            }
        }
    }
    let mut rest = stack;
    (left > 0 && rest.set_count(left).is_ok()).then_some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_is_saved_and_read_back_past_a_byte_s_slots() {
        let mut item = Compound::new();
        item.insert("id".into(), Tag::String("minecraft:stone".into()));
        let mut items = vec![None; 108];
        items[3] = Some(item.clone());
        items[107] = Some(item);
        let back = decode(&encode(&items).unwrap()).unwrap();
        assert_eq!(back.len(), 108);
        assert_eq!(back[107], items[107]);
        assert_eq!(back[3], items[3]);
    }

    #[test]
    fn an_older_save_s_byte_slots_are_read() {
        let mut item = Compound::new();
        item.insert("Slot".into(), Tag::Byte(5));
        let data: Compound = [("Items".into(), Tag::List(vec![Tag::Compound(item)]))].into();
        assert!(decode(&data).unwrap()[5].is_some());
    }
}
