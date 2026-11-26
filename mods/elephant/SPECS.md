# Elephant: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): a large savanna elephant you can ride
with a friend, fit with a dyeable saddle blanket and a double chest on each flank. It fears mice supplied by a separate
future mod; Elephant will not require that mod.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm. Numbers are starting values, to balance in game.

## 0. Status

In development (0.1.0, not released), steps 1 to 4 of 5:

- **Step 1**: the elephant as an entity (`elephant:elephant`, its spawn egg), 2.2 × 2.6 blocks (hit where its trunk and
  tail reach too: the library's `creature::pick`); drawn with its adult model, walking with a four-legged gait
  (diagonal legs together), its ears fanning with a wide flap now and then, its trunk swaying, its tail flicking, its
  head following what it looks at; its gear hidden; its spawning (savanna and savanna plateau, weight 2, groups of 2
  to 4, on open, about flat ground, not in water); its charge (`charge.rs`: hit, it runs at its attacker within 8
  blocks, strikes once for 4 damage and throws them back, then lets them go, 3 seconds before the next); its guide
  page (uncommon); its sounds (`sources/sounds.toml`, cut from CC0 freesound recordings: trumpets from a zoo's and a
  wild elephant's, grumbles, hurt cries, a long low call as it dies, heavy steps). Its spawn egg is our own drawing.
- **Step 2**: its own stats (the library's `creature::stats`, moved from the Vulture: 40 to 60 health, 0.8 to 1.2 times
  its speed); taming (`tame.rs`: sugar cane a player throws near it, eaten, one chance in four a piece, hearts or
  smoke; the guide's "Tamed"); tamed, it follows its owner on foot (the library's `friend::Follow`, moved from the
  Sporelins) or stays on an empty-handed right-click, never despawns, and a hay bale from its owner heals 10 health.
- **Step 3** (`gear.rs`): its owner fits a vanilla saddle, drawn as its blanket (the `saddle` group in
  `elephant_saddle.png`'s neutral colours), dyed in place with any of the 16 dyes (saved on it, `elephant:gear`);
  saddled, an empty-handed right-click gets on: its owner in front (whoever sat there moving behind), another player
  behind its owner only (alone, they are told the owner rides in front); two seats (`passenger_attachments`). Its
  owner steers (the library's `creature::ride`, moved from the Vulture): it turns toward where they look and walks
  forward, back and sideways at its pace, stepping up a block; with another in front, it stands. Whoever hits one of
  its riders is charged as if they hit it (the library's `anger::provoke`). Its saddle drops when it dies. The guide's
  "Saddled", "Coloured", "Rode" and "Two riders".
- **Step 4** (`gear.rs`, the library's `creature::pack`, moved from the Vulture): its owner fits two chests at once
  on a flank (holding at least two; the left first), drawn as the `chest_left` and `chest_right` groups, a 54-slot pack
  a side (`creature:pack`). Its owner's sneak + right-click opens its screen: a gear page (its saddle, a click taking it
  off and its riders down; staying or following; a chest per fitted side, a click opening that side's page, a
  shift-click taking its chests off with what they hold) and a 6-row page per side. Its saddle, its chests and all
  they hold drop when it dies. The guide's "Packed" and "Both sides".

Not tried in game yet.
- **The adult model** (`sources/models/elephant.bbmodel`; [front](../../docs/models/elephant-elephant-front.png) and
  [back](../../docs/models/elephant-elephant-back.png) previews): a broad grey elephant with ears attached at the
  temples, grey on their front surfaces, tilted slightly back and shaped with a rounded, heart-like outline on the pixel
  grid. It has two ivory tusks, a long, tapering segmented trunk, four pillar legs and a slim tail ending in a dark tuft.
  Its torso is extended, and its skin has pixel folds.
- **Texture**: the 64 × 64 pixel-art texture has a grey-brown palette and crease markings; the final art pass is
  approved.
- **Baby model** (`sources/models/baby.bbmodel`; [front](../../docs/models/elephant-baby-front.png) and
  [back](../../docs/models/elephant-baby-back.png) previews): compact and round, with a larger head, proportionally
  broad ears, shorter legs and trunk, and no tusks. Used from birth through 10 minutes.
- **Juvenile model** (`sources/models/juvenile.bbmodel`; [front](../../docs/models/elephant-juvenile-front.png) and
  [back](../../docs/models/elephant-juvenile-back.png) previews): an intermediate size and build, with a longer trunk
  and legs and its first small tusks. Used from 10 to 20 minutes.
- All three models share the elephant's 64 × 64 texture. The baby and juvenile models have no visible saddle blanket or
  chests; only adults can be ridden or equipped.

Next: (5) breeding, the baby and juvenile stages, its drops (meat, the ivory tusk), the ceremonial
trumpet, the guide's facts and items, fleeing mice.

## 1. Technical context

- **Mod id**: `elephant` ✅. Entity `elephant:elephant`, spawn egg `elephant:elephant_spawn_egg`.
- **Dependency**: the series' library `creature` ([common/SPECS.md](../../common/SPECS.md)).
- **Mouse support** ✅: a separate future mod will provide mice. Elephant's mouse response is optional and must work
  without a hard dependency on that mod.
- **Languages**: source text in English; in-game text in English and French.
- **Models**: Blockbench free-format cube models, converted with `cargo creature model elephant` for the adult and
  `cargo creature model elephant baby` / `cargo creature model elephant juvenile` for the young stages.
- Uses from the library: model drawing and sounds, hitbox sizing, ownership and the creature guide's page. Elephant owns
  its saddle, coloured blanket, chests, storage and two rider seats.

## 2. The elephant

- **Appearance** ✅: a sturdy African savanna elephant with muted grey-brown, wrinkled skin and grey ears attached at
  the temples. Their rounded, stepped outline curves slightly back and narrows toward the head in a heart-like shape.
  It has two ivory tusks, a segmented trunk tapering toward its tip, thick legs and dark toenails. The longer model is
  about 2.25 blocks wide, 2.6 blocks tall and 3.75 blocks long, including its trunk and lowered, tufted tail; the
  starting hitbox is 2.2 × 2.6 blocks.
- **Spawning** ✅: uncommon in savanna and savanna plateau, with spawn weight 2, in groups of 2 to 4, on open, mostly
  flat ground. It does not spawn in water or on steep slopes. The weight is relative to other natural spawns.
- **Behaviour** ✅: an elephant does not attack unprovoked. If it or one of its riders is attacked, it makes a short
  charge at the attacker within 8 blocks, dealing 4 damage and moderate knockback, then does not pursue it. If attacked
  again, it may charge again after a 3-second cooldown. When a mouse from the separate mouse mod comes within 8 blocks,
  it trumpets and flees for 5 seconds; its riders stay mounted.
- **Taming** ✅: throw sugar cane near a wild elephant, following the Vulture and Seagulls' food pattern. Each piece has
  a one-in-four chance to tame it for the player who threw it. A tamed elephant follows its owner or stays on right-click,
  and does not despawn.
- **Variable stats** ✅: each elephant rolls maximum health from 40 to 60 HP and walking speed from 0.8 to 1.2 times
  its base speed of 0.16 blocks per tick, following the Vulture's system ([vulture/SPECS.md](../vulture/SPECS.md) §2).
  At birth, each baby inherits each stat as the average of both parents' values and one random value from the species'
  range, divided by three, exactly as on the Vulture. Stats are hidden from players, as on the Vulture.

## 3. Riding and equipment

- **Saddle and two riders** ✅: a tamed elephant has no blanket until equipped. The owner fits a vanilla saddle; it appears
  as a woven blanket and permits up to two riders. Any player may mount. Only the owner may use the front seat and steer;
  other riders use the rear seat as passengers.
- **Dyeable saddle blanket** ✅: use any of the 16 vanilla dyes on the equipped saddle to recolour the blanket in place.
  There is no separate carpet item. Render the `saddle` group with a neutral-colour texture tinted by its saved dye, so
  the elephant's body texture stays unchanged. A saddle is required to ride.
- **Double chests** ✅: the owner fits two vanilla chests together on each flank, four chest blocks total. Each side has
  a separate 54-slot inventory, 108 slots in total. No chest is visible until that side's pair is fitted. The
  `chest_left` and `chest_right` groups appear with their respective pairs.
- **Equipment screen** ✅: the owner uses sneak + right-click, following the Vulture's equipment screen. It manages the
  saddle and both chest inventories; the exact slot layout follows during implementation.
- **Death drops** ✅: when the elephant dies, its saddle, all four chest blocks and both inventories' contents drop, as
  with the Vulture.

## 4. Breeding and growth

- **Breeding and young stages** ✅: two tamed elephants breed with hay bales. They produce a baby, tamed to its owner,
  that grows through a baby stage for 10 minutes and a juvenile stage for 10 minutes, then becomes an adult. Feeding a
  young elephant hay bales speeds up growth. Parents have a 5-minute breeding cooldown. Babies inherit their parents'
  stats as described in §2. Only adults can be ridden, following the Vulture's system ([vulture/SPECS.md](../vulture/SPECS.md)
  §2 and §8).
- The baby and juvenile models are visual stages only; their age transitions, stats and gameplay are specified above
  and remain to be implemented.

## 5. Drops and additional mechanics

- **Equipment drops** ✅: the saddle, all four chest blocks and their contents drop on death (§3).
- **Natural drops** ✅: only adults drop 1 to 3 raw elephant meat. Adults also have a 5% base chance to drop an ivory
  tusk before Looting; Looting increases the tusk chance. The tusk can drop from wild or tamed adults. Babies and
  juveniles drop no meat or tusks. Meat cooking and the exact Looting scaling can follow the Vulture and Toucan patterns,
  with numbers balanced during implementation.
- **Ceremonial trumpet** ✅ (`elephant:ceremonial_trumpet`): craft with one ivory tusk, one copper ingot and one string.
  Right-click to play the elephant's trumpet sound. It is purely for sound and does not command elephants. It is
  reusable, has no durability, and has a 3-second cooldown.
- **Adult healing** ✅: feeding a tamed adult a hay bale restores 10 HP.
- **Food**: sugar cane is used for taming; hay bales are used for breeding, growth and adult healing.

## 6. Bestiary ✅

The page records the biomes where a wild elephant was met, and these facts:

- **Tamed**: befriended an elephant.
- **Saddled** and **Coloured**: equipped the saddle, then dyed its blanket.
- **Packed**: fitted a double chest on either flank; **Both sides**: fitted both.
- **Rode** and **Two riders**: rode an elephant, then rode with a second player.
- **Bred** and **Raised**: bred elephants, then raised a young one to adulthood.

The Items tab lists the ivory tusk and ceremonial trumpet.

## 7. Out of scope for v1

- More than two riders.
- The mouse entity itself; it belongs to the separate future mouse mod.

## 8. In-game validation

After implementation, test in a new survival world with the series library and Elephant enabled:

- Find uncommon groups of 2 to 4 on open, mostly flat ground in savanna and savanna plateau; check the hitbox against
  trees, doors and slopes, and confirm it does not spawn in water or on steep slopes.
- Check that an elephant charges attackers of itself or its riders within 8 blocks, dealing 4 damage and moderate
  knockback without pursuing, with a 3-second cooldown between charges; check that a hay bale restores 10 HP to a tamed
  adult. Also check that it flees when a mouse is nearby.
- Check baby, juvenile and adult models from the front, sides and back: the baby has no tusks; small tusks appear in
  the juvenile stage. Also compare the head, trunk, ears, feet and tail as the elephant grows.
- Verify that a wild and an unsaddled elephant have no equipment. Saddle it, recolour the blanket, then ride as owner and
  non-owner; only the owner should steer.
- Fit a double chest on each side; check both inventories save, reopen and drop correctly, including on death.
- Check individual stats, breeding, inherited stats and growth through baby, juvenile and adult stages.
- Confirm the ivory tusk's 5% base chance applies to both wild and tamed adults, never to babies or juveniles.
- Confirm babies and juveniles drop no meat, and adults drop 1 to 3 raw meat.
- Craft and use the ceremonial trumpet; check its sound, 3-second cooldown and that repeated use does not wear it or
  change elephant behaviour.
- Bring a mouse close and check that it trumpets and flees without dismounting its riders. Confirm the detection range
  and flight duration; Elephant must behave normally when the mouse mod is absent.

## 9. Open questions

- ~~Q1: The creature and its look?~~ A broad-eared, tusked savanna elephant with a long trunk (§2).
- ~~Q2: What makes it useful?~~ A two-seat ride, dyeable saddle blanket and one double chest on each flank (§3).
- ~~Q3: Where does the mouse come from?~~ A separate future mod. Mouse integration is optional; Elephant must not require it (§1).
- ~~Q4: Where and in what groups does it spawn?~~ Savanna and savanna plateau, uncommon, weight 2, groups of 2 to 4, on
  open, mostly flat ground; not in water or on steep slopes (§2).
- ~~Q5: How is it tamed?~~ Throw sugar cane near it; each piece has a one-in-four taming chance (§2).
- ~~Q6: What happens when a mouse is nearby?~~ Within 8 blocks, the elephant trumpets and flees for 5 seconds; its
  riders stay mounted (§2).
- ~~Q7: How are inherited stats calculated?~~ Each baby stat is the average of both parents' values and a random
  value from the species' range, divided by three, as on the Vulture (§2).
- ~~Q8: How does breeding and growth work?~~ Hay bales breed elephants and speed up growth; young elephants spend
  10 minutes as babies and 10 as juveniles, with a 5-minute breeding cooldown. Only adults can be ridden (§4).
- ~~Q9: Which natural drops should the elephant have?~~ 1 to 3 raw elephant meat and a rare ivory tusk with a 5% base
  chance before Looting (§5). Looting scaling and meat cooking follow the series pattern, with exact numbers tuned in game.
- ~~Q10: Which bestiary facts should be kept?~~ Keep Tamed, Saddled, Coloured, Packed, Both sides, Rode, Two riders,
  Bred and Raised (§6). No mouse-flee fact.
- ~~Q11: How does the elephant defend itself?~~ If it or a rider is attacked, it charges the attacker within 8 blocks
  for 4 damage and moderate knockback, then does not pursue; it can charge again after 3 seconds (§2).
- ~~Q12: Which food, if any, heals a tamed adult?~~ A hay bale restores 10 HP to a tamed adult (§5).
- ~~Q13: What is the ivory tusk used for?~~ It crafts a ceremonial trumpet that plays the elephant's trumpet sound and
  does not command elephants (§5).
- ~~Q14: What recipe and use details should the ceremonial trumpet have?~~ One ivory tusk, one copper ingot and one
  string; right-click plays the elephant trumpet sound, reusable without durability, with a 3-second cooldown (§5).
- ~~Q15: Which elephants can drop an ivory tusk?~~ All adults, whether wild or tamed; babies and juveniles never drop
  tusks (§5).
- ~~Q16: Do baby and juvenile elephants drop raw meat?~~ No. Only adults drop 1 to 3 raw elephant meat (§5).
