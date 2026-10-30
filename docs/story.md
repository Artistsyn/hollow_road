# Hollow Road: story and world plan

Adapted from the author's unfinished novel *Veil of the Shattered Realms*. The full digest
of the book (16 chapters) was read for this. This plan stays within what the book says and
marks every invention as **[new]**.

The hero is nameless and is not Olekeen. They walk the road Olekeen walks in the book,
through the same places, fights and people. Because the game is open, they can take that
road in their own order, go off it, and come back to it. The book stops when the Order
captures the party; the game supplies what comes after.

---

## 1. The shape (decided 2026-10-28)

**Olekeen and his party are in the world, a step ahead of the hero.** The book's last scene is
the Order gassing them at the Spire and carrying them off. The game's finale is the nameless
hero coming in to save them, and the world. All of the book's cast enter this way: as people
whose road keeps crossing the hero's.

**The hero fights the book's bosses.** At each key battle the book's threat splits in two (it
often does already): Olekeen's party takes one half, and the hero takes the boss.

- **Why the hero is on this road at all.** At Hollowmere's well, Olekeen pries one rune from
  the stones and the hero another; the well is ringed with them ("they sang once, before the
  mists came").
  - **They answer each other:** the two runes hum toward one another, so the hero's rune pulls
    them along Olekeen's trail.
  - **They answer the hero:** Helena's words, "your rune wakes what sleeps", are said to the
    hero.
- **Why the hero, not Olekeen, ends up facing each boss:**

| Battle | Olekeen's party | The hero |
|---|---|---|
| Hollowmere beast | Olekeen on the rooftops, flung down by its whisper; Atrael thrown into a shack | Finishes it, with Atrael's burning gaze opening its core |
| East gate bone beast | At the pits, fighting the bandits (the same evening in the book) | Lurz's alarm brings the hero to the gate |
| Blackridge camp | Pinned by the crossbowman on the ledge | Takes the chief |
| The hollow's void horror | Krix and Lurz are its prey | Arrives in time, or does not (§4) |
| Shrine guardian | Holds the stone mural door with Exodus while the guardian regrows | Breaks its eyes, the rune as bait |
| Varnholt's edge, then the cathedral | Digs the thirty survivors out of the cellar | Goes into the cathedral after the priest |
| Eryndor | Holds off the tear-spawn in the plaza | Takes the statue apart, then the archangel |
| The Spire | Arrives as the Star-Wraith falls | Fights the Star-Wraith; the Eye is made whole, and Exodus springs the trap on everyone |

- **The trap at the Spire.** The gas takes the hero too: they wake at the last shrine, a
  death in all but name. The Order has the Eye, Fin-S and Olekeen's party. The last act
  follows them to the Order's stronghold.

**The book's own words.** It is the author's book. Its dialogue and its descriptions of places
and people are used as written wherever they fit, and in its voice where they need adapting.

**Reference art (the author's Hero Forge designs)**, for the sprites:

| Who | Look |
|---|---|
| **Olekeen** | Goblin, pale yellow skin, long ears, red hooded leathers with dark straps, a red recurve bow with spines on its limbs |
| **Atrael** | Wings of gold fading to red, a full iron-grey mask with red eyes, flame-coloured hair, gold and olive armour, red-and-white tabard, a round-bladed shield with a red jewel; a greatsword on his back (not visible in the art, but the author's design has it): he fights with the greatsword |
| **Exodus** | White rabbit-folk, black plague-doctor mask with brass studs and a red lens, black gi with red trim and red sash, wrapped fists |
| **Steel** | Steel grey and silver armour, a closed flat-faced great helm with an eye slit (no beak: the author's correction), a flared pistol, a tall rectangular shield, a red cape |
| **Fin-S** | A wooden automaton: carved wood with moss-green patches, jointed limbs, blocky fists, violet eyes and violet lights at the wrists (the shuriken slits) |
| **Helena** | No design exists: draw her from the book's description alone (grey cloak and low hood, frost-pale eyes, rune-like scars on her palms) |

**Enemies and bosses** take their look from the book's descriptions of what Olekeen fought
(the well beast, the bone beast at the east gate, the void horror in the hollow, the
Blackridge crew, the hounds in the mire). New enemy types are added where a region needs
one the game lacks, in the same spirit.

### The opening (decided 2026-10-29)

The opening tells the world's myth and the hero's condition, and nothing of the plot. It
follows how the Souls games open:

- **Dark Souls** narrates the age before, the fire that is fading, and the curse the player
  bears, then drops them in a cell.
- **Elden Ring** tells the Shattering and shows the demigods only as legends. It calls the
  player one of the Tarnished and gives a single vague goal.
- **Bloodborne** explains almost nothing: a cryptic contract, "seek paleblood", and a waking.

What they share: myth rather than plot; the great threats spoken of as rumour; the player
defined by a condition, not a past; one cryptic errand; every named person kept for the
road. Each line must still be true to the book and set up the plot the game follows. The
opening is about 72 s (`Cutscene::opening` in `src/cutscene.rs`):

| Shot | Where | Says | What it sets up |
|---|---|---|---|
| 1 | The Spire at dusk | Before the Shatter, a Veil of runes walled the world off from the hunger beyond; the old stones sang and it held | The Veil and Voryn's hunger (§7 of the digest), without naming him; the runes |
| 2 | The angels' ruins | Cloaked mortals turned the Eye that Sees Beyond on the Veil and tore it; their names were not kept | The Eye; the Order's guilt, as a hint only (they wear cloaks; nothing names them) |
| 3 | The same, closer | The angels broke the Eye in three and set a guardian over each piece; they have kept their watch too long | The three shards and the rotted guardians: the Angel Priest, the archangel, the Star-Wraith ("I guarded… too long") |
| 4 | The marsh at night | The tear never closed; the Taint seeps through it: mist, whispers, the dead who will not stay fallen | Why the world fails now; the Taint; the game's risen dead |
| 5 | Hollowmere's gate | Something has come up the old well; the village hears it whispering | The first bounty's beast and its voice, as rumour |
| 6 | Varnholt | The cathedral bells toll like a dirge, then stop; the priest swears to guard what lies beneath | The first shard and its guardian, as rumour ("I shall guard it, always") |
| 7-8 | The first road, on the hero | Most of those the Taint takes rise hollow; a few rise whole, with only scraps of who they were, and hear the old stones calling. "You are one of these." | The hero's condition; the prologue's memory is one of those scraps; the pull toward the runes |
| 9 | The Warrens | The hunters' guild pays for proof of any kill; the road begins there | The first step: Gruk's board |
| 10 | The guild hall | HOLLOW ROAD, "Seek the stones that sang." | The errand: the runes |

**The hero's condition:** one of the few the Taint does not hollow. They keep scraps of a past
(the prologue's old soldier) and hear the rune-stones. That is why the Hollowmere well's rune
answers them, and it is what Helena means by "something anchors you" (the book leaves the anchor
unexplained; the game keeps it a question). The **Order, Exodus, Olekeen's party and the
creation myth's names** are not in the opening. The player meets them on the road. "Seek the
stones that sang" is answered when the hero pries the rune from the Hollowmere well.

### How the story is told (decided 2026-10-29)

As the Souls games do it: almost nothing is said outright. The world is told through what is
carried, what people let slip, and what the places show.

- **Item lines** (`src/lore.rs`). Every piece, remnant, key and shard has two or three
  sentences, shown at the shrine under the piece picked and on the new **Keepsakes** screen.
  They say where a thing came from and who held it, and they leave a thread hanging.
- **The keeper and the smith move on with the road** (`src/voices.rs`). After each region
  boss falls, each has news, told once, then a few idle lines until the next boss. They never
  explain; they let things slip.
- **Rumour before sight.** A place is spoken of before it is reached (the opening, Maren, Gruk),
  so arriving at it pays something off.
- **Every line must be true to the book, or a thread marked [new] below.** Each must also make
  sense to a player who has not read the book.

### Hidden threads [new]

These run under the book's plot. They are hinted, never told, and each can be dropped if the
author disagrees.

| Thread | What hints at it | Pays off |
|---|---|---|
| **Maren, the keeper, was one of Helena's order** (perhaps the sister Helena "lost to shadows"). The order cut runes into their palms; the Taint took them; Helena ended them. Maren came back whole, as the hero did. | She keeps her hands in her sleeves; her line about "women in grey" with runes cut into their palms; "if you find a woman in grey there, do not tell her where I keep my fire"; the Rimed Sabre and Frostweaver's Focus lines ("the frostweavers do not say who ended it") | When Helena is met, she can recognise a description of the keeper, or the keeper's fire. |
| **Brannoc made Fin-S**, for the Order, and set him free ("live free", the voice Fin remembers). | "I forged for an Order once"; "I carved, once, before I took up the hammer"; the marks "cut into the wrists of something I made"; "if you ever meet a carving that talks, tell it the old man says to live free" | When Fin-S hears the hero say "live free". |
| **The Order made the masks and the gas, and tore the Veil.** | Brannoc: the Shrine's carving of cloaked figures was "stamped at the top of every order I was paid to fill"; "I made their masks. Beaks, to keep the gas out"; Maren: "something with a mask", "those who tore the Veil wore cloaks" | The Spire: the gas. The stronghold: the book's hint made plain. |
| **The Order's sign is a sun behind a veil**, scraped off what its soldiers left behind. | The Kite Shield | The Order Agent, Exodus, the stronghold. |
| **The ones who rise whole** keep scraps of a past and hear the rune-stones. The hero's wraps hide burn-marks "too regular to be accidents". | The opening; the Leather Wraps; Maren: "the mist gives back what it cannot hold"; "you were never going to be whole"; "whether it is you that it gives back" | Left open, as the book leaves Olekeen's "anchor" open. |
| **Bounty's fox mark.** | The Hunter's Shortbow (the mark on everything he gave away); the Ring of the Quiet Step | Bounty's trail (§4, side quest 9). |

### Exploration rewarded (decided 2026-10-29)

The story keeps its order where the plot needs one. Everything else is open, and the world rewards
whoever finds it first. This follows the Souls games:

- **The story's gates stay shut until their time.** Keys, the Spire's slab and Varnholt's ward
  keep the chronology the plot depends on.
- **Places off the main thread are open before the story names them.** Each holds something
  only an early comer gets: usually a beast, and from it a unique piece (a drop, or a remnant
  Brannoc forges).
- **The world goes on without the hero.** People act when the story reaches a beat. A bounty the
  hero has not taken is taken by someone else, and what is theirs is not the hero's.
- **People answer what the hero did, and in what order.** The one who would have told how they
  felled a beast instead complains that someone beat them to it.
- **Missable, but the main thread never breaks.** Going early earns the unique piece. The story
  reads the same without it.
- **Side quests the way FromSoft writes them.** People met in several places over the game,
  their state moving on with the hero's progress and choices. They are told in pieces and can
  be missed (as Siegward, Solaire or Hyetta can).

**Built (`src/visitors.rs`): the East Gate.** It lies off Stilt Lane through a lashed arch, at
the edge of the mire, and is open from the start. The book's bone beast is there: **the Keening
Cage**, with violet-lit ribs, an eyeless antlered skull and barbed tendrils, its shadow-hounds in
front of it.

| | The hero went out early | The story's order |
|---|---|---|
| The beast | Fought; leaves the **Violet-Veined Bone**; Brannoc forges the **Keening Antler** (a spear; +25% to the open, strikes cheaper) | Once Gruk pays the Hollowmere bounty, Olekeen kills it (`World::story_moves`); nothing of it for the hero |
| Gruk | Pays 400 for the bone, unposted ("Board or no board, a dead thing's a dead thing"); at the Hollowmere pay: "Olekeen went out anyway. He'll be sore." | At the Hollowmere pay: "Olekeen went after it. That yellow one don't wait for the board." |
| Olekeen at the gate | "Bone thing... dead? ...You kill? You? Hmph. Good kill. Olekeen not say twice. Was Olekeen's bounty. Next one, Olekeen first." | "Too late. Olekeen kill. Big boom. Dead now. ...Your stone hum? Olekeen's hum at bone thing. Like they talk." |
| Maren, Brannoc | News after the bone: the road noticed; Brannoc wants the antlers | Nothing |
| The rune | Hums toward the gate only while its beast stands (after Hollowmere, the book's next hunt) | |

Olekeen stays at the husk while the hero does, then moves on, a step ahead, to Blackridge.

**Built: the Black Water** (`atlas.rs` "black_water", `visitors.rs` Krix). Krix's beast lies on
past the East Gate, where the mire goes black among old stones (the book: "old stones, black
water", and beyond the gate "the mire and reeds stretch dark and still"). It is **the Mother of
the Black Water**: a hunched drowned thing with the faces of those it took pressing out of its
body. It does not eat them. It keeps them, and speaks in their voices; named, it whispers in
Mara's: "Krix...? Krix, is that you? It's so cold down here..." The book never kills it, and no
one is sent after it: the rune does not hum toward it (`Boss::sought`).

| | Krix (the guild hall, from the Collector's fall until he goes out with the Blackridge bounty) |
|---|---|
| Not yet killed | Tells the tale: Jek went under first, then Mara; "It didn't eat them. It kept them." Mara's whistle at night. "Dead don't mean gone. You'll hear yours too." |
| Killed after his tale | "...Did it talk? In her voice? Don't. Don't tell me." With the whistle: "Keep it. She'd want it blown by someone still walking." Without: "If it's still out there, I can't go back for it." |
| Killed before anyone told the hero | "Heard someone went out past the gate... Wasn't me. Should've been me." With the whistle, he knows it: "Mara's. That's Mara's." |
| The whistle brought back later | "...You went back for it." |

- **Rewards:** **Mara's Whistle** (a talisman, +12% damage, stamina back 10% faster), among the
  Mother's dead at the far end; and her **Hooked Finger**, which Brannoc forges into the
  **Drowner's Hook** (a hook-blade sword: 5% of each strike back as health, rolls cheaper).
- **Maren and Brannoc** each have a beat for it, told last and kept apart from the story's idle
  lines (`Speaker::talk`).
- **Where (decided 2026-10-29):** past the East Gate, not off the vale. The book puts the mire
  beyond the gate and Krix's beast "in the mire east of the vale"; one place serves both. It also
  asks an early comer to get past the Keening Cage first, or arrive after Olekeen.
- **Krix's own fate** is side quest 3, built 2026-10-29 (the Fog Hollow, below).
- **Ordering checked (2026-10-29):** nothing closes the Black Water. Krix's talk only reads
  whether the Mother is dead, and the Keening Cage opens the way whether the hero or Olekeen
  kills it (`visitors::tests::krixs_beast_waits_whatever_order_the_cage_krix_and_the_chief_come_in`).
  Krix's lines are what can be missed: he leaves the guild hall when the chief falls.

**Next, in the same pattern, from the book:**

1. **The Drowned Fingers** (Tusk, then Vren): whoever reaches the ruin first finds the
   rune-marked blade. If the hero did, Tusk never sells it on, and Vren has nothing to show; he
   asks to see the hero's instead. **[built 2026-10-29: the Drowned Fingers lie north off the
   Marsh Edge, a mud path through leaning stones, open from the start; their far end meets a
   ladder up from the Drowned Mill, kicked down from below (a shortcut opening from the Mill).
   The **Rune-Marked Blade** is at the end until Gruk pays the Hollowmere bounty; then the shaken
   hunter has found it (`World::story_moves`). **Tusk** at his Warrens cart, once the hero
   carries the rune, tells either way. **Vren** waits down a side path off the Warrens, at **the
   Broken Spire**: he shows the blade Tusk sold him, or asks to see the hero's and pays for the
   look with **Vren's Chipped Ring** (+30% echoes, 8% more damage taken: he does not trade
   straight). Once the east gate's bone is in hand he reads it and points west to Stonehollow.]**
2. **The bandit cave** under Blackridge. **[built 2026-10-29: "The Bandits' Hole", a side path
   off the Bandit Hollows. The crew's stash, with two of them guarding the chest, holds the
   **Caravan Signet** (a ring, +15% echoes, +8% damage). Its seal is a sun behind a veil, the
   mark scraped off the Kite Shield: the caravans the crew hit were the Order's. Once their
   chief falls, the crew run off with whatever is still there (`World::story_moves`): the chest
   stands open and bare. At the Blackridge pay Gruk tells which: "Somebody got there first...
   Guild don't pay for what it don't know," or "Ran there with the take when their chief went
   down." The shortcut back to the Pine Road is dropped (2026-10-29): Ridge Camp's shrine is one
   road from the cave, so a way out to the Pine Road would save no walking, and the Pine Road
   already has its three roads (the Cliff Path's rope is that shortcut).]**
3. **Olekeen, a step ahead**, as a questline: the East Gate, Ridge Camp after the chief,
   Stonehollow's registrar, the Hollowed Shrine, then captured at the Spire. Each meeting answers
   whether the hero beat him there. Friendly enough, and he can be called to a boss. **[built
   2026-10-29: at Stonehollow's gate he answers the hollow (both saved, Lurz lost, both lost, or,
   while they are still in the fog, "Go back. East, off the hollows"); on the Sunken Steps he is
   first to the Hollowed Shrine or beaten to it again ("Olekeen stop keeping score"); under the
   Spire's sealed slab he is found bound by the Order's masked men and cut free. Calling him to a boss is built (2026-10-29, `src/summons.rs`): asked to hunt together at
   Stonehollow's gate (only if both hunters came home) or once cut free under the slab, he answers
   the humming stone raised at a veil, from Stonehollow on, except while the Order holds him.]** **[Ridge Camp built 2026-10-29: once the Blackridge bounty is up he waits there for dark
   to go up at the chief ("Olekeen here first this time"); if the hero already killed the chief,
   "You first to bone thing, now you first to chief. Olekeen keep score." Either way Krix and
   Lurz go east with him, to the hollow, which leads into side quest 3.]**

## 2. The main thread, region by region

Each step names the **pointer**, the person or thing that tells the player where to go next.

### 0. The Ashen Warrens (tutorial)

- **The guild hall.** The book's "squat, ramshackle hovel marked by a rusted blade over
  the door", reeking of sweat and sour ale: a low timber hall with tables and benches,
  barrels, notices nailed to the walls and framed doorways to back rooms. The Debt Collector,
  the guild's own enforcer, now hollowed, holds it.
- **Gruk's counting room (a dead end).** A door in the hall's end wall leads into it. **Gruk**
  is barricaded behind his counter, his bounty board on the wall behind him. He is one-eyed,
  wiry and snarling: "Shut that door. The hollowed don't knock." and "No proof, no pay."
- **The way ends at Gruk's counter.** The wall closes the room, the walk stops, the
  dead-end notice shows with TURN, and E speaks to Gruk.
- **Gruk's board** holds the bounty that starts the game: *"Beast in Hollowmere.
  Tentacles. Screaming. 50 silver."*
- **Pointer:** Gruk ("Hollowmere's past the vale. Bring proof."), plus the board.
- **Fight from the book:** the east gate's bone beast is an optional encounter here (§4).

### 1. The Wretched Vale and Hollowmere

- **The Hollowmere Screamer is the book's well beast.** It speaks into your head ("Little
  arrow, so small, so frail…") and opens a second maw.
- **[new] Atrael's arrival.** If Olekeen is in the world, Atrael arrives in a burst of
  ravens partway through the fight, as he does for Olekeen. He fights for a phase, then is
  flung into a shack.
- **The rune.** After the beast falls, the hero pries the **rune** from the cursed well.
  The old woman says the runes "sang once, before the mists came."
- **What the rune does:** it is the game's compass (§5), and the start of the real story.
- **The Screamer no longer gives Shard I** (§6).
- **Pointer:** Gruk pays for the proof, plus one coin for the rune: "trouble follows stuff
  like that." Gruk sends the hero to **Tusk** and **Vren** about the rune, and posts the
  **Blackridge bounty**.

### 2. Blackridge

- **The Blackridge crew.** Ten silver a head, twenty for the chief, proof required. It runs
  through the ridge camp, the bandit hollows and the Chief's Hold.
- **The hollow.** This is where the void horror kills Krix and Lurz (§4, Krix and Lurz).
- **Pointer:** Vren: "past the Blackridge… Stonehollow… someone driven hoards relics."

### 3. Stonehollow

- **The guild registrar** (scar-browed, "Solid take, blue-eyes") pays for proof, the same as
  Gruk. She sends the hero to **Steel's Occult Research and Pawn** in the north end.
- **Steel** is a soul bound to empty armour. He reads the rune and the relics as "a trail".
  He sends the hero to **Helena Thornheart** in the Underborough chapel.
- **The Underborough alley.** The rune flares and a hooded thing strikes, then melts away.
  It is a stalker who returns later (§4).
- **The Hollow Magistrate** stays the city's boss.
- **Pointer:** Helena: the Hollowed Shrine, "in the Veil's heart," holds "your blight's root,
  your rune's maker."

### 4. The Hollowed Shrine

- **The guardian.** A stone-and-shadow guardian fixates on the rune. The rune can be thrown
  as bait to open it up, a fight mechanic taken from the book.
- **Exodus Lightfoot** of the Order of the Radiant Veil appears here. He is a helpful,
  cold, rabbit-folk monk in a plague-doctor mask, and can be summoned for this fight.
- **The heart mural.** It opens to the hero's **blood**: pay health, and the door opens.
  Beyond it, the altar tells of the Eye that Sees Beyond, split into three shards.
- **Pointer:** Helena recognises the Varnholt cathedral in the carving: "three days' ride
  west… you'd hear its bells from the hills."

### 5. Varnholt

- **Varnholt's edge.** The villagers have mutated, and the charred-well monstrosity rises.
- **Atrael burns barns with people hiding inside.** The player chooses whether to stop him.
  This is the game's first mercy choice, and it follows the hero (§8).
- **The cathedral.** The priest swears to guard the piece, then becomes the **Angel
  Priest**. His freed spirit gives the **First Shard of the Eye**.
- **Mira** is in The Broken Bell tavern.
- **Pointer:** Mira, who gives the map to Eryndor through the Blackwood.

### 6. Eryndor and the Blackwood

- **The Blackwood:** tear-twisted wolves, and the spider tree.
- **Eryndor:** the **Statue Archangel** is taken apart knot by knot, as in the book.
- **The crypt:** the corrupted archangel on its throne gives the **Second Shard**.
- **Fin-S** is found asleep in the roots of a lone tree, between the Shrine and Varnholt
  (§4).
- **Pointer:** the freed spirit says to go north, "where starlight meets stone."

### 7. The Celestial Spire

- **The way there:** the Void-Twisted Forest, the Starlit Wastes and the Shattered Sky
  Peaks. Their bosses are the radiant construct and the warped drake.
- **The Sealed Slab** opens when the two shards come near, as in the book.
- **The guardian angel** becomes the Star-Wraith. It holds the **Third Shard**.
- **The Eye made whole.** The three archangel spirits tell how the Veil was made: Lirathar,
  Sylvara and Korath walled Voryn away.
- **[new] The Order strikes.** Exodus's Crimson Veil operatives gas the room and take the
  Eye, and Fin-S if he is with you. The finale is the Order's stronghold (§8). **[built
  2026-10-29, "The last act" below]**

---

## 3. The cast

The keeper and the smith stay the game's own characters (Maren and Brannoc for now).
Everyone below is from the book.

| Who | Where met | Role in the game |
|---|---|---|
| **Gruk** | His counting room off the guild hall, Warrens | Bounty board, pays for proof. Sends the hero to the next lead. Built 2026-10-29 (`src/guild.rs`). |
| **Tusk** | Warrens market | Trader. Talks about the Drowned Fingers and Vren. Gives bread, which heals a little. Built 2026-10-29 (a goblin; no bread yet, as there are no consumables). |
| **Vren** | Broken Spire, a new side path off the Warrens | Buys relics and pays in lore and items. Not to be trusted. Built 2026-10-29: reads the rune and the bone, and pays one ring for a look at the hero's blade; the full relic trade (flesh, quill, rubbings) is still to come. Also built 2026-10-29 (`src/relics.rs`): once he has seen the rune he buys the Screamer's voice (900), the cage's bone (800) and the Mother's finger (1400, and his loupe: echoes ×1.18) for coin and what each tells him; a relic sold is gone from the smith. |
| **Skif** | East gate | A youngling guarding a husk: "No touch, no take." Built 2026-10-29: there once Olekeen has been out to the cage; grown into a hunter after Stonehollow, he gives the hero his rusted knife (side quest 6). |
| **Krix, Lurz** | Guild, east gate | Haunted hunters, and a questline (§4). Both are goblins: Lurz "a wiry goblin with a patchy beard" and a spear; Krix never named so, but clawed, scarred, one ear a torn stump (the book; redrawn as a goblin 2026-10-29, after a first human sprite). Krix drinks in the guild hall from the Collector's fall until Blackridge, and answers the black water (built 2026-10-29, `src/visitors.rs`). |
| **Olekeen** | Roadside, shrines; first at the East Gate | The other hunter, a step ahead. Gives lines and hints, and can be summoned for some bosses. Met first over the Keening Cage, his kill or the hero's (built 2026-10-29, `src/visitors.rs`). |
| **Atrael** | Hollowmere, then Varnholt's edge | Arrives in ravens. A summon, and a mercy choice. Built 2026-10-29: at the Hollowmere Screamer, once Olekeen is in the world, his ravens pour into the square uncalled and he fights beside the hero without making the boss stronger, until he is thrown out at half its health. |
| **Registrar** | Stonehollow guild | Pays for proof, like Gruk. |
| **Steel and Azrael** | Stonehollow north end | Pawnbroker and relic reader. Buys relics. The **[new]** Steel questline (§4). |
| **Helena Thornheart** | Underborough chapel | Heals madness (clears Taint), trains the frost school. |
| **Exodus Lightfoot** | Hollowed Shrine | An ally first, then the betrayer. Leads the Order agents. |
| **Mira** | The Broken Bell, Varnholt | Historian. Her map points to Eryndor and then the Spire. |
| **Fin-S** | Lone tree, shadowed vale | A wooden automaton. Questline (§4). |
| **Rik, and the old woman** | Hollowmere | Rik is a young man with a bandaged arm (the book's "young Rik"); the old woman is his gran's age. Lore, and the madness that lingers after the beast. Rik listens to the well (built 2026-10-29, side quest 5). |

**Voices to keep:**

- **Olekeen:** third person, no articles ("Olekeen not done").
- **Steel:** booming jokes ("Gets 'em every time!").
- **Helena:** dry and cutting.
- **Exodus:** speaks in absolutes.
- **Fin-S:** stretches his syllables when the Taint is near.

---

## 4. Side quests

Each one opens encounters that would not happen otherwise. Several open optional areas or
shortcuts.

1. **The Blackridge bounty** (Gruk).
   - **Steps:** take the proof sack. Collect bandit helms on Blackridge, and the Chief's
     spiked helm.
   - **Where to cash in:** at Gruk, or at any registrar.
   - **Reward:** silver (echoes), plus **the Registrar's Writ** **[new]**, which opens the
     guild's stock to buy from.
   - **Follow-up:** the old trader's warning, "more bandits hidin'," opens an optional
     **bandit cave** under the ridge **[new area]**. It ends in a shortcut back to the Pine
     Road.

2. **The east gate** (Lurz's alarm, after Hollowmere). **[built 2026-10-29 as "Exploration
   rewarded", above: the Keening Cage, open from the start; Olekeen takes it once the story
   arrives. Skif is still to come.]**
   - **The fight:** the bone beast with violet-lit ribs, and its shadow-hounds. This is an
     optional boss on a new side path, **the East Gate and the Mire**.
   - **Drop:** a **bone shard** that hums against the rune.
   - **Skif** is left guarding the husk.

3. **Krix and Lurz** (the guild, then Blackridge). **[built 2026-10-29: the Fog Hollow, a dead
   end off the Bandit Hollows (`src/hunters.rs`). Once the chief is down they are in its fog
   with **the Horror in the Hollow** (the book's formless void horror, "Small hunters... in the
   fog... all break..."). Lurz waits at its mouth ("They're still there!"). Each time the hero
   falls there the horror takes one, Lurz first, then Krix; if Stonehollow's court falls first,
   both were left to it; killed before the chief, it never had them. Survivors: Krix at the guild
   gives **Krix's Notched Axe**; Lurz at Stonehollow's Lantern Market gives **Lurz's Spear**.
   The horror leaves **the Horror's Eye-Crack**, forged into **Ripper's Tendril**. Whoever died
   returns as a Taint echo, a duel drawn from his own art tinted violet: **the Echo of Lurz** on
   the Violet Road, **the Echo of Krix** at the Blackwood's edge, each leaving the weapon he would
   have given alive (`hunters::ECHOES`).]**
   - **How they join:** they sign on for the Blackridge bounty and walk ahead.
   - **In the hollow:** the void horror waits for them. If the hero reaches it in time, the
     fight is to save them; one or both can live.
   - **If they live:** they turn up later. Krix gives the notched axe and Lurz the spear,
     both unique.
   - **If they die:** they return much later as **Taint echoes**, optional duels with lore
     ("Dead don't mean gone… you'll hear yours too").

4. **Vren's relics.**
   - **Steps:** bring him what the dead leave: the beast flesh, the bone shard, the hound
     quill and rune rubbings.
   - **What he pays:** each relic buys lore and a reward.
   - **The last step:** he points to **the Drowned Fingers** **[new area]**, a ruin north
     past the vale ("all mud and old stones"). It is where the rune-marked blade came from.
   - **Rewards there:** a unique catalyst, and a shortcut down to the Drowned Mill.

5. **What the well drew up** (Hollowmere).
   - **Trigger:** after Blackridge, Rik hears the whisper "Frail… yet…" in the wind.
   - **The fight:** at night the well stirs again. **Krix's mire beast**, never killed in the
     book, rises.
   - **What it opens:** the **mire east of the vale** **[new area]**.
   - **[2026-10-29: the mire and its beast are built as the Black Water, past the East Gate and
     open from the start ("Exploration rewarded"). The well's stirring after Blackridge is still
     to come: if the Mother still stands, it is the night she reaches up the well; if she is
     dead, Rik hears only a whistle, twice.]** **[built 2026-10-29: Rik at Hollowmere, after its
     beast ("You hear it still?"); after Blackridge the well calls "Krix, it's so cold" while the
     Mother lives, and whistles twice once she is dead.]**

6. **Skif's watch.** **[built 2026-10-29: "Skif's Rusted Knife", a talisman.]**
   - **What happens:** come back to the east gate after Stonehollow. Skif has kept his post
     and grown into a hunter.
   - **Payoff:** this pays off the book's "Olekeen come back." It is a small reward, and a
     line that lands.

7. **Steel's flesh** **[new, from his wish in chapter 16]**.
   - **Steps:** he asks for relics from each region and tells his experiment's story piece
     by piece.
   - **At the Spire:** the player can let him touch the Eye. That is a cost, a temptation
     and an ending factor.
   - **Built 2026-10-30** (`src/steel.rs`). Once he has read the rune, each region boss beaten
     (the Magistrate, the Wraith Lord, the Tainted Priest, the Statue Archangel) is a relic he
     asks to see, and a piece of his story: the name Richald Steel; the circle of scholars who
     sought to cheat death, one of whom laughed like Krix; the circle taking his flesh; Azrael
     found half-dead in a dig site and rebuilt; and his wish to feel flesh again through the
     Eye. With the first two shards carried he waits at the Sealed Slab and asks to hold them.
     - *Let him* gives **Richald's Signet** (+12% damage, +20 health), lets two Taint in, and
       at the end he is flesh: a grey man on the Lower Stairs who complains of the cold, his eyes
       going violet some nights **[new]**.
     - *Refuse* and he stays tin, and leaves the door out of his story.

8. **Fin-S, Unit 0.**
   - **Waking him:** bring a carving knife, or the rune.
   - **What he does:** he follows the hero to shrines and carves a gift for each companion
     met.
   - **His mystery:** his inscription reads "MUST REMAIN UNFINISHED." The Order wants him.
   - **His fate:** protecting him is the long thread into the finale.

9. **Bounty's trail** **[new, the longest questline]**.
   - **The trail:** across the world, old bounty notices are signed with a **fox mark**, left
     by Bounty, the vanished fox-folk archer Olekeen idolised.
   - **The end:** in the Void-Twisted Forest, Bounty's echo.
   - **Reward:** **Bounty's Longbow**, a two-handed bow, unique. This is where the weapon-set
     idea pays off.
   - **Olekeen:** if he is in the world, he learns what became of his hero, through the
     player.
   - **Built 2026-10-30** (`src/bounty.rs`). Four fox-marked notices, each shown only once the
     one before is read: Stilt Lane (rats, paid in bread for the lane), the Vale's edge (the
     marsh lights, children brought home), the Old Sewers (forty silver sent home; then
     wraiths out of Eryndor took a Warrens girl, a goblin, yellow-eyed, and he went after
     them), and the Ruins of Eryndor (never posted: "Whoever finds this, take the bow").
     - The echo waits in **the Void-Twisted Forest** (`void_wood`, built 2026-10-30): the
       Blackwood gone wrong, a dead end off the Blackwood Edge, which now forks at its end. The
       last notice, in Eryndor, points back there. It sits in region 6; §2.7 lists it on the way
       to the Spire, with the Starlit Wastes and Shattered Sky Peaks (built, "The road to the
       Spire" below).
     - **The Echo of Bounty**: an archer duel, tinted violet like the other echoes. An arrow
       at the chest, a long draw, a low arrow, a clubbing bow strike (parry it). Measured:
       the reader bot wins 12 of 12 at tier 6 with 66% health left (the Statue: 10 of 12,
       56%).
     - **Bounty's Longbow** is an off-hand bow: a heavy arrow down the lane, flat and far.
     - Olekeen, at the Sealed Slab, learns the girl was his sister **[new]** and that Bounty
       did not forget them. He gives the bow back: the note said whoever finds it.
     - The ending tells of bread on Warrens doorsteps again, under a burned fox head.

---

## 5. Guidance: how the player knows where to go

- **Bounty boards** at the Warrens guild and the Stonehollow registrar. A board holds the
  current main bounty plus any open side bounties, each one line, vague but placed:
  "Beast in Hollowmere."
- **The rune is the compass.** This is the book's own device.
  - **At forks:** it hums toward the branch that leads sooner to the next region boss still
    standing (the nearest region first), which is where the story goes on: "The rune hums
    toward the left." When both ways are as near, it says nothing. **[done 2026-10-29:
    `World::rune_hums`; and 2026-10-29 the soft pulse on the hero, a green glow traced round them
    for four seconds that beats and fades (`Pilgrimage::pulse_rune`)]**
  - **Elsewhere:** it warms near danger, and flares near tears and relics.
  - **Control:** it can be turned down or off for players who want none of it.
- **People who point on.** Each NPC has a line for every milestone, pointing at the next
  place without a map marker:
  - Gruk → Hollowmere
  - Vren → Stonehollow
  - Helena → the Shrine
  - Mira → Eryndor
- **Item descriptions.** Every relic, remnant and shard says where it came from and what it
  answers to.
- **The keeper's talk** moves on with the hero's progress, giving one rumour per region.
- **Dead ends are told.**
  - **What is drawn:** the scene shows a wall, a counter or a closed door.
  - **What is shown:** when the walk stops, a notice appears ("The hall ends here.") along
    with the TURN prompt.
  - **Planned for every dead end:** the guild hall, the Wraith Sanctum, and the Summit.

---

## 6. Changes to the world map this needs

- **The Screamer drops the Rune, not Shard I.** The rune is a new key; nothing is gated on
  it yet. **[done 2026-10-29: `Key::Rune`, "Hollowmere Rune"; an older save past the Screamer
  is given it on loading]**
- **The shards come from the book's three places:**
  - **I** from the Angel Priest (Varnholt)
  - **II** from the Eryndor archangel
  - **III** from the Star-Wraith (the Spire)
- **[done 2026-10-29]** The Angel Priest leaves the First Shard, the Statue Archangel the
  Second, the Archangel the Third, and the road up the Spire wants the first two.
- **The shard gate moves.**
  - **Today:** the Court of the Statue needs all three shards.
  - **New:** the **Sealed Slab** opens to shards I and II, as in the book.
  - **The court:** opens to the **Second Shard**, or is left open with the Statue Archangel
    guarding it.
- **The heart door** at the Hollowed Shrine opens for blood, paid in health. It becomes the
  Shrine's way on, and the Wraith Lord's Sigil keeps its current place.
- **New places:**
  - **side paths:** the East Gate and the Mire, and the Broken Spire (Vren)
  - **optional areas:** the bandit cave, the Drowned Fingers, and the mire east of the vale
  - **a shortcut** back from each of those three
- **Each new place needs:**
  - a scene, made in PathForge
  - an entry in the atlas
  - an entry in the journey
- **How places meet (2026-10-29).** Each area keeps its own time of day and weather. At a
  crossing, the area ahead first shows lit, fogged and skied as the one being left. Over the
  last 10 m before the threshold it turns into its own (PathForge `LIGHT_BLEND_M`,
  `Ctx::lit_as`). Its window is seen through the near area's whole air, fog banks included,
  so it never shows as a dark or bright box hanging in the fog. A hall ahead (roofed) keeps
  its own lamplight and air, as an interior does at any hour. The Tainted Cathedral is a
  roofed nave (9 m vault) entered by great doors (`Via::Door(2.6, 5.5)`): from the Blackwood
  and from Varnholt it stands as a 16 m brick church front, not a walled street. Arches and
  gates always clear the walking camera (PathForge `ARCH_CLEARANCE_M`); none stands in a
  fork's mouth or just before a doorway.

---

## 7. Mini-boss rewards

Mini-bosses leave a lesser unique, from a chest behind them or as a drop:

| Mini-boss | Lesser unique |
|---|---|
| **The Debt Collector** | **The Collector's Tally** **[new]**: a talisman; after a parry, the next kill pays double echoes. In a chest behind him in the guild hall. |
| **The Pack Alpha** | a quill-spined trinket |
| **The Order Agent** | the beaked mask, which resists gas |
| **The Rune Sentinel** | keeps the Gravewarden's Slab chest it already guards |
| **The Butcher** | **[new]** |
| **The Void Stalker** | **[new]** |

The big bosses keep their remnant-to-weapon recipes.

---

## 8. The finale and endings

The book stops at the capture. The game's last region is **the Order's stronghold**
**[new]**:

1. The hero follows the Order to where Olekeen's party is held.
2. Fin-S is "Unit 0."
3. The Order intends to use the Eye to do again what the cloaked ritualists once did. The
   book hints that the Order was behind the first tearing of the Veil.

The design doc's four endings stay. They are shaped by:

- which tears were sealed
- whether Atrael was stopped at Varnholt's edge
- whether Krix and Lurz lived
- what became of Steel and Fin-S
- the hero's Taint

**The Taint as a stat (built 2026-10-30, `src/taint.rs`).** It is worked out from the save,
not stored:
- The heart mural paid in blood lets in one; holding the shards out for Steel lets in two.
- Each one adds 5% damage and takes 6% off what a flask heals. The status line shows "taint N".
- Helena, once met, sees each source and offers to draw it out, once. Kept is kept.
- At two or more, the three guardians do not answer: *Call the three names* is not offered.
- At the end, Helena lights a candle for a hunter who carried it out of her chapel.

| Ending | What happens |
|---|---|
| **Seal the tears** | Call Lirathar, Sylvara and Korath to mend the Veil. |
| **Let the Veil fall** | Voryn's hunger comes through. |
| **Give the Eye to the Order** | "Harmony, as one." |
| **Break the Eye** | The world endures with its tears. |

---

## 9. Where the book contradicts itself, and the choice made

| Seam | Taken |
|---|---|
| Order of the Radiant Veil vs Crimson Veil | Radiant Veil is the order's public name; the Crimson Veil are its operatives |
| Atrael's master: the Shroud Sovereign or The Mother | The Mother, who gave him fire and ravens; the Shroud Sovereign is what the Hollowmere villagers call her |
| Azrael's origin | Found half-dead at a dig site and rebuilt (the last telling) |
| Helena's past | Her order guarded the living with runes; the Taint took them and she ended them. Her sister and her village were lost in that |
| "Myrren" | Dropped; Steel is the relic-hunter |
| Shard count at the fusion | Three shards, one per guardian |

---

## 10. Build order

1. **The guild hall as a real place.**
   - the hall's end drawn **[done 2026-10-29: a timber hall, a door, Gruk's counting room]**
   - the dead-end notice **[done]**
   - Gruk and his board: the Hollowmere bounty, then the Blackridge bounty **[done]**
   - the Debt Collector's chest **[done 2026-10-29: the Collector's Tally]**
2. **The rune.** **[done 2026-10-29]**
   - the Screamer drops it
   - it hums at forks
   - item descriptions (`src/lore.rs`, for every piece, remnant, key and shard)
   - Gruk's coin for it
   - also done: the keeper's and the smith's lines move on with each region boss
     (`src/voices.rs`); bosses whisper as they are named and speak last words as they fall
3. **People who point on**, with milestone lines: Gruk, Tusk, the keeper.
4. **The Broken Spire and Vren**, and the relic trades.
5. **Krix and Lurz, and the east gate.**
6. Region by region after that, each with its pointer, its people and one side quest.

## Calling help to a boss (built 2026-10-29)

`src/allies.rs` (the fight) and `src/summons.rs` (who, where, and how it is earned).

- **Never explained.** Each summon is earned by something the hero says back, hinted in the
  words around it, and shows only as a prompt at the veil: E calls, the answer is told, and the
  ally walks into the yard from the boss's far side.
- **Krix:** at the guild, home from the fog, he offers his axe. *Take the axe* gives the weapon;
  *Tell him to keep it* gives nothing, and he says "Mara blew twice when she'd found the trail...
  Where the dark's thickest, that's where the trail goes." With Mara's whistle carried (taken
  from the Black Water), "Blow Mara's whistle twice" shows at a boss's veil. Never in the fog or
  the black water, nor past region 4. The axe or the ally: never both.
- **Olekeen:** *Hunt with me* at Stonehollow's gate (both hunters saved), or under the Spire's
  slab once cut free (a second chance). "Loud stone, Olekeen hear": "Raise the humming stone"
  at veils from Stonehollow on, the rune pulsing as it is raised; not while the Order holds him.
- **How they fight:** across the boss from the hero, so what it turns on one it turns its back
  on the other. Krix up close, Olekeen at range with arrows. They strike only in an opening
  (the boss open, recovering, or busy with the hero), see most blows coming and get clear (Krix
  7 in 10, Olekeen 8), keep off when the boss winds up at them or they are hurt, and follow the
  hero's lead: while the hero is not hurting the boss they hold back. The boss's eye goes
  between them by who has hurt it lately, never three blows running at an ally.
- **Not too strong:** the boss takes 40% more with help. Measured (`allies::tests`, 24 bouts a
  boss): fights are shorter and the reader bot loses less health over the Chief, Magistrate and
  Butcher together; a masher still loses (at most 1 in 24); a hero who only guards cannot be
  carried (at most 2 in 24).
- **Exodus** (built): *Fight beside me* in the Vine Halls; "Let the stone sing" at the Wraith
  Lord's veil, and nowhere else. After the Summit he is the enemy, and never answers again.
- **Olekeen at the end:** taken again in the trap, he answers once found in the stronghold's
  cells, Exodus's fight included.
- Still open: Atrael's raven arrival during the Screamer fight (§2).

## The later cast (built 2026-10-29)

`src/cast.rs` (who, where, what they say), `src/alley.rs`, `src/hollowed.rs`.

- **Stonehollow.**
  - The registrar pays for the Blackridge helms if Gruk has not (and every proof payment now
    reaches the purse: Gruk's never did).
  - She sends the hero to Steel, who reads the rune as a trail and sends them under the city.
  - In the Old Sewers the rune flares and **A Hooded Thing** strikes with no veil, then melts
    away at 60% of its health. It is the Void Stalker.
  - Helena knows that it struck, and names the Shrine.
- **The Hollowed Shrine.**
  - Exodus waits in the Vine Halls with his offer.
  - The heart mural at the fork wants 40% of the hero's whole health (never the last of it) and
    stays open after (`Key::Heart`).
- **Varnholt.**
  - Atrael burns the barns: *Stop him* or *Let it burn*. Mira's map follows, and her words
    depend on the choice.
  - Fin-S sleeps in the roots on the violet road: *Touch the rune to its chest* or *Let it sleep*.
  - Woken, Fin-S carves a gift for each companion met: the Carved Hunter (Olekeen) and the
    Carved Listener (Krix).
- **Sprites drawn from the book's few words.** The Registrar's and Mira's sprites follow the
  book's scant description; please check them. Helena's is from her description, as planned.

## The last act (built 2026-10-29)

`src/finale.rs` (the trap, the stronghold's people, the Eye's choice, the fates), `src/order.rs`
(its two fighters), `Cutscene::trap` and `Cutscene::ending`. **[new]**: the book stops at the
capture, so everything after the Summit is written for the game in the book's voices. Please
read it as a draft.

- **The trap.**
  - The Archangel falls, and out of the white the Eye is made whole.
  - The three voices name Lirathar, Sylvara and Korath, and add the one hint the game gives:
    "Where it is torn, it can be closed. Close enough of it, and we will hear you."
  - Olekeen comes up the stair. Exodus steps out of its shadow: "The Radiant Veil takes back
    what it lent."
  - Gas, and Fin-S carried off as Unit 0 if he was woken. The hero wakes at the Sealed Slab's
    fire.
- **The Order's Stronghold** (region 8, west of the Summit along the map's top):
  - **The Crimson Stair:** the Summit's far side, in snow, under the Order's banners.
  - **The Radiant Gate:** the last fire, and **the Crimson Captain**, a lance and gas pots.
    Its rope bridge back to the Sealed Slab is a shortcut.
  - **The Penitents' Cells:** a gauntlet. Olekeen is found here ("Leader the rabbit").
  - **The Unit Foundry,** a dead end off the cells: rows of faceless wooden units, and Fin-S on
    a plinth, to be "finished tomorrow". He is there whether or not he was woken: the Order took
    him from his roots if the hero left him. *Live free.* (Brannoc's words, the hidden thread
    paid off) or *Come with me.*
  - **The Chamber of the Eye:** **Exodus Lightfoot**, who says what the book hints: "We tore it
    once, in cloaks."
- **Voryn and the Prophet (the author, 2026-10-29).**
  - The cloaked ones' first use of the Eye let **part of Voryn** through the tear.
  - That part is the Taint. It has ridden the Order since as **"the Prophet"**, whose word is
    "harmony, as one". The Order never names Voryn; the hero has the name from the guardians.
  - **The Order made Fin-S as the Eye's conduit**, to bring Voryn's world wholly into this one
    rather than only opening a rift. That is what "MUST REMAIN UNFINISHED" guards.
- **The last fight has two phases, and the second depends on Fin-S.**
  - **Phase one** is always Exodus himself, not possessed and not strengthened (the author,
    2026-10-29).
  - **Freed before Exodus falls** (either answer in the foundry), Fin-S is no door. The Prophet
    takes Exodus instead ("Not all of me, then. Enough."), and the fight goes on against
    **Voryn's Vessel**, a monstrous Exodus.
  - **Left on his plinth**, Fin-S is finished by the Eye, and Voryn comes through him whole
    ("All of me, at last"). The fight goes on against **Voryn Made Whole**, Unit 0 grown into
    blades. It is the harder fight, and Fin-S's fate is told as lost.
  - **The only hint** is Olekeen in the cells: "Rabbit say: wooden one is door. Door for
    Prophet."
  - **Dying** in either second phase sends the hero back to face Exodus again. The chamber's
    echoes and **Exodus's Red Lens** (forged into the **Veil-Piercer**) come when the second
    phase falls.
- **The endings.** Olekeen hands over the Eye and asks the hero to choose:
  - *Call the three names* (**the Veil mended**): offered only to a hero who beat three of the
    five things that came through tears (the Keening Cage, the Black Water's Mother, the Fog
    Hollow's horror, the Void Stalker, and the Warped Drake nesting in the Peaks' tear), with the
    Taint under its limit.
  - *Break it* (**the world endures, torn**), *Give it to the Order* (**harmony, as one**) and
    *Open the Veil* (**Voryn comes through**).
  - Then what became of everyone, as the road left them:
    - Atrael stopped or not.
    - How many of Krix and Lurz lived.
    - Fin-S freed, following, or finished and broken in the chamber.
    - Steel: tin, flesh, or the one who refused.
    - The hero's Taint, and Bounty's trail followed to its end.
  - The pilgrimage begins again, and the last act is played again in it.
- **Fights measured** (`side::boss_bots`, 12 bouts each; the masher and the turtle never win):
  - The Captain: the reader bot wins 2 of 12 (the Archangel's figure).
  - Exodus: the reader wins 4 of 12.
  - Voryn's Vessel: 4 of 12.
  - Voryn Made Whole: 2 of 12.
- **Built 2026-10-29, late:**
  - The Shrine guardian's rune bait: in the side fight with the wraith, "Throw the rune" (E,
    or the Use button) lures every eye after it for two seconds; the rune comes back in fifteen.
  - Atrael's ravens at the Screamer (the cast table).
  - Vren's relic trade (the cast table).
  - The Order Agent's Beaked Mask, in a cache in the Bridge Ward: Brannoc's make, it lets a
    third of the Order's gas through.
- **Built 2026-10-30:** Steel's flesh and the Eye-touch choice (side quest 7), Bounty's trail
  (side quest 9), and the Taint as a stat (§8).

## Text and causality pass (2026-10-30)

Every line of dialogue, narration, caption and item text was read against this plan, the
atlas and each line's gate; then every side quest was traced for cause and effect.

- **Text:** about 70 fixes, mostly wrong compass directions against the map, tense, names that
  differ from the atlas titles (Bridge Ward, Pine Road, the Old Sewers for "the Underborough",
  the Lower Stairs for "the Lower City"), counts that disagreed (seven went into the black
  water: Jek, Mara, four more and Krix), and lines that assumed a route the gates allow
  differently (Olekeen's "first this time", Steel's carry-the-relic wording, Fin-S's "again").
- **Causality, fixed:**
  - The registrar's court purse paid on every word with him: now once. All meeting gifts are
    given on a first hearing only.
  - Vren showed the rune-marked blade while it still lay in the Drowned Fingers.
  - Fin-S could be woken after the trap and then be missing from the foundry. His tree, his
    carvings and Exodus's thanks now end with the trap (unless Fin-S was saved).
  - Olekeen stood at the East Gate and Ridge Camp long after the story moved on; he could be
    at the Sunken Steps while tied under the slab; he could be called to the Archangel and
    then "arrive late". His gate, camp and shrine meetings now close in time, and he is never
    called at the Summit.
  - Olekeen left tied at the slab: the trap now shows him still tied there, not climbing the
    stair. Left in his cell: he chews free and hands over the Eye with a line saying so.
  - The horror killed before Blackridge: nobody speaks of a rescue that never happened.
  - Krix and Lurz lost while the hero was at the court: Olekeen says so at the Sunken Steps.
  - Stonehollow's registrar, Steel, Helena and side quest 7 no longer hang on the optional
    Blackridge chief; Steel follows the registrar's pointer.
  - Helena met after the Shrine no longer sends the hero there; her Varnholt pointer stops
    once the cathedral is done. Her end candle shows only if she met the hero.
  - Mira blames only a hero who let the barns burn; one who never stood before Atrael gets a
    plain version.
  - Atrael's ravens at the Screamer were unreachable in a first pilgrimage: they now come once
    Gruk has posted the beast, and never for a hero who stood against Atrael.
  - Gruk paid for a bounty already done only on a second visit; now on the first. With the
    east gate's bone sold or forged, the gate watch's word stands for it.
  - New pilgrimages: dead Krix no longer drinks at the guild; Exodus is not called again; the
    heart's Taint can be drawn again; Bounty's fate holds while his bow is carried.
  - Bounty's fate tells of Olekeen only if Olekeen was told (at the slab, or in the cells).
- **For the author to decide:**
  - **The heart mural is not a choice.** The Wraith Sanctum needs the Heart key and the
    Sanctum is the only way to Varnholt, so every hero carries a point of Taint until Helena
    draws it. Either say so, or open another way.
  - **The Order ending** has the Order kneel for the Eye just after the hero killed what it
    called its Prophet.
  - **Olekeen's sister** (Bounty's trail) and Steel's violet-eyed flesh are inventions.
  - Atrael is named on the boss bar when his ravens come, before he introduces himself.
  - The Keeper's and smith's beats after the Summit were written for a Summit ending and say
    nothing of the gas or the captured party.
  - Helena's book line "Three days' ride west, past the court" became "east" to match the map.


## Shrine glass and fair fights (built 2026-10-30)

**Flasks.** Three to start, fifteen with every piece of shrine glass (`src/vials.rs`). Each piece is a key item, melted into one more flask at the next rest, and kept into a new pilgrimage.

- **The story's five** come from the great foes on the road: the Chief's Hold, the Magistrate's Court, the Wraith Sanctum, the Tainted Cathedral and the Statue's Court.
- **The thorough's three** lie in chests at the ends of roads no one has to walk: the Drowned Mill, the Bandits' Hole and the Void-Twisted Forest.
- **The brave's four** are on foes no one is sent after: the Black Water, the Fog Hollow, the Rune Vault and the Whispering Altars.

**Fairness rules, for every duel (`src/side.rs`).**

- **No true combos.** A string's next blow starts late enough in its windup that a hero reeling from the last one is free with `ANSWER` ticks to roll, or the string ends. A boss's rest after a hit also lasts at least the hero's reel.
- **No windup under 0.4 s (`QUICKEST`), enraged or not.** That is a reaction to see it begin, and the rest to answer.
- **A leap's ground warning is where the walls let it land**, not where it aimed.
- **Tier scaling counts half the forged armour**, so upgrades are felt, and grows 16% a tier instead of 22%. A late boss's ordinary blow takes about a third of an expected hero's health, and its big telegraphed ones about half.

**Parry and perfect block.**

| Off hand | Parry, live ticks | Perfect block, ticks |
|---|---|---|
| Buckler | 13 | 11 |
| Duelist's Parma | 19 | 8 |
| Kite Shield, Warden's Bulwark | 11 (slower to raise) | 14 |
| No shield (blade deflect) | 9 | — |

**Tuning yardstick.** `side::tests::boss_bots` adds a Human bot to the frame-perfect Reader. It sees a windup 15 ticks (250 ms) late, times its answers up to 3 ticks off, and carries the flasks expected at the tier (`Glass::expected_at`). It still carries no rings, extra levels, boss weapons, arts or allies, so it understates a real player late on.

## The road to the Spire (built 2026-10-30)

The way from the Court of the Statue to the Sealed Slab, region 7 (`src/spire_road.rs`, `src/atlas.rs`).

- **The Starlit Wastes** (`starlit_wastes`, shrine at 3.0): glassy ground and fallen obelisks under the stars, off the Court of the Statue.
  - Its boss is **the Radiant Construct**, the angels' keeper of the old pilgrims' road: a palm strike (parry it), a slam that breaks guards, a low sweep, starglass shot from afar, and a radiance that rings out at range.
  - Its remnant is the Starglass Core, which the smith forges into the **Starglass Maul**. The core's lore line says it let the masked ones by: the construct knew the Order's sign.
  - **Olekeen's mark**, cut into a fallen obelisk: OLEKEEN FIRST, and an arrow north. Heavy boots all going the same way follow his arrow.
- **The Shattered Sky Peaks** (`sky_peaks`): the pass where the sky broke.
  - Its boss is **the Warped Drake**, nesting in the tear: a bite string, a tail sweep, a dive, and violet breath.
  - It is the fifth tear (`finale::TEARS`). Its remnant, the Drake's Warped Fang, becomes the **Warped Fang** spear.
  - His last mark breaks off mid-name (OLEKEEN FI...). Smashed gas pots lie under it, the air smells sweet like rot, and drag marks lead up to the Spire Stair. This is where the Order's gas took him, before they tied him under the Slab.
- **The Spire Stair** now climbs from the Peaks to the Sealed Slab.
  - The Slab's way stays barred until both shards of the Eye are carried (`Gate::Keys`).
  - A shortcut opens from the Stair back down to the Starlit Wastes' shrine.
- **Dead ends rewarded.** Chests with unique pieces: the Unit Foundry (Maker's Chisel), the hollow among the graves (Gravedigger's Ring), and the Void-Twisted Forest (Yellow-Glass Bead). Three dead ends also hold shrine glass (see "Shrine glass and fair fights").
- **Both bosses** are full region bosses, with last words for the construct and the drake counted among the beasts. The keeper and the smith each have a new beat for them.
- **Boss art:** PixelLab, side view, 9-frame clips under `assets/pixellab/radiant_construct` and `assets/pixellab/warped_drake`, captured in `shots::boss_moves` (m32–m35).
