# Future: all seven current public categories and native design decisions

> Dated reference-mechanism research and design context. Current executable acceptance is recorded only in [Rusty validation](../VALIDATION.md).

On 2026-11-04 the official [Future homepage](https://futureclient.net/) was inspected in the in-app browser. Clicking every public category selector revealed **141 current module contracts**: Combat 22, Exploits 17, Movement 26, Render 26, World 14, Miscellaneous 28 and Other 8. The former frozen 884-occurrence matrix only captured the initial Combat panel plus historical changelog mentions; it was not an exhaustive current Future inventory. Keep that baseline frozen and use [the supplemental current register](implementation-analysis-future-current.json) for this newly observed scope.

All seven panels' descriptions were read. These panels expose feature descriptions, not complete settings or internal code. Existing changelog settings are historical, not proof of every current mode. The linked v3.0 full changelog is [login/permission gated](https://www.futureclient.net/forum/showthread.php?tid=60319); no account access was attempted. Proprietary internals and runtime behavior remain unavailable. The 141 rows below are **original native design proposals and discriminating tests**, not recovered Future algorithms. Open-source family analysis elsewhere is separate evidence.

## Combat

| Feature | Original native design and individual acceptance requirement |
|---|---|
| AntiBots | Track disclosed identity/profile/entity consistency with uncertainty and a user override. Test a real player with delayed profile data and a server NPC sharing normal metadata; no missing field proves bot status. |
| Aura | Bounded eligible-target ranking, actual native reach/LOS/cooldown and owned normal rotation/attack. Test friend target crossing a wall, weapon swap, active food use and invalidated target just before dispatch. |
| AutoArmor | Component-aware equipment suitability/protection with one ordinary acknowledged equip transaction. Test curse-bound armor, glider preference, full cursor, custom wearable slot and incoming inventory update. |
| AutoBowRelease | Release a currently owned normal bow use after configured simulation charge, with ammo/component validation. Test charge below minimum, manual release, hand change, menu opening and server-cancelled use. |
| AutoCrystal | Native damage/exposure preview plus normal placement/attack queue under self/friend thresholds. Test partially occluded explosion, occupied support, no crystal, immunity state unknown and delayed placement acknowledgment. |
| AutoLog | Ordinary leave action from explicit received-health/threat policy with session identity and debounce. Test transient local damage prediction, unknown health, absorption option and reconnect before delayed leave. |
| AutoPot | Full potion-effect policy, self-impact prediction and normal rotation/use/slot lease. Test harmful mixed effects, poor landing geometry, manually changed slot and refused cooldown. |
| AutoSoup | Treat stew as food under ordinary mechanics; a server plugin healing capability is separate configuration. Test full hunger/low health on normal server, bowls, soup depletion and manual use. |
| AutoTotem | Serialized ordinary offhand equip with cursor/menu/session preconditions and availability feedback. Test last totem outside hotbar, offhand food ownership, delayed update and death before completion. |
| AutoTrap | Bounded target-relative shell proposals, exact collision/support/material checks and one normal acknowledged placement. Test moving target, protected friend, partial block, no reachable face and rejected placement. |
| AutoXP | Ordinary experience-bottle use at current view, bounded stock/rate and mending policy if enabled. Test player moves under low ceiling, last bottle, manual scroll, item cooldown and armor not eligible for repair. |
| BowAim | Native read-only projectile simulation plus bounded target interception/rotation. Test moving shooter, water, low bow charge, blocked arc and target that changes direction. |
| Criticals | Normal critical-eligibility HUD and optional grounded jump timing; accepted damage decides result. Test riding, climbing, levitation, attack recharge and server correction. |
| FeetTrap | Explicit foot-shell placement preview with material/escape policy and native explosion estimate. Test incomplete wall, fluid/partial cell, friend overlap and self collision; no universal blast-safety claim. |
| HoleFill | Bounded received hole classification and reachable ordinary fill actions, with target/friend/own-hole exclusions. Test double hole, missing neighboring chunk, water, trapdoor and entity entering before placement. |
| KeepSprint | Ordinary sprint intent after attack, respecting hunger/use/ability state; distinguish intent from preserving momentum. Test native attack slowdown, food, collision and received sprint state. |
| NoHitDelay | Crosshair attack scheduling should use actual native cooldown and miss policy without extra synthetic dispatch. Test rapid miss-to-hit, menu open, held button and target beyond true bounds. |
| Replenish | Component-aware slot role memory and normal inventory transfer after count threshold/depletion. Test main-hand totem consumption, selected source protected, incompatible components and cursor occupied. |
| SelfFill | A self-cell placement preview must respect native entity collision and mark unsupported placements. Test block intersects actual body, permission denial, partial shapes and unreceived target. |
| SelfTrap | Reachable shell executor with headroom/escape and material count preview. Test player posture change, ceiling, another feature moving player and cancelled support placement. |
| SprintReset | Optional normal sprint release/repress after confirmed ordinary hit, with input lease and no promised knockback. Test missed attack, active shield, physical sprint held and hit acknowledgment delayed. |
| ThrowPot | User-invoked bounded one/two-potion sequence selected from actual health/effects and ordinary use. Test first potion heals enough, second unavailable, target surface changes and manual cancel between uses. |

## Exploits

| Feature | Original native design and individual acceptance requirement |
|---|---|
| AntiHunger | Food planning and ordinary movement-efficiency information; retain authoritative hunger/saturation changes. Test sprint/swim/attack costs and full hunger with low saturation; no damage immunity or server cost suppression claim. |
| ChorusControl | Show bounded possible chorus landing space and current use status; normal fruit consumption retains actual server teleport. Test unknown destination chunks, unsafe landing, manual cancel and no deterministic destination promise. |
| ClickTP | Click selects an ordinary waypoint-walk target; actual teleport requires a native permissioned action. Test far/unloaded target, border, wrong dimension and nonoperator. |
| ColorSigns | Local sign-format preview and ordinary editor action within actual accepted formatting rules. Test server strips color, waxed sign, front/back and text-length boundary. |
| Disabler | No portable native equivalent for breaking another server's simulation checks. Offer correction/protocol diagnostics using disclosed state. Test software remains unknown when no authoritative version marker exists. |
| ExtendedFirework | Ordinary rocket scheduling with actual active-boost lifetime and stock/cooldown; no forged lifetime extension. Test expired rocket, damaged glider, duplicate use and remote boost updates. |
| FastProjectile | Native center-line launch/damage estimate and ordinary charge timing under actual projectile mechanics. Test speed cap, shooter movement, collision and accepted damage rather than asserting increased motion. |
| FastUse | Native item use cadence and consumption/charge distinctions; historical protocol behavior is not a host feature. Test bow/food/shield/throwable separately with real cooldown and component duration. |
| NewChunks | Method-specific received-data classifier with confidence, session scope and first-seen-age alternative clearly named. Test old chunk loaded first, misleading palette/fluid data and cross-dimension reuse. |
| NoMineAnimation | Local mining animation customization can preserve ordinary action/state synchronization. Test actual mining progress remains visible in diagnostics and third-party visibility is not presumed. |
| Orebfuscator | Search only disclosed received block facts and label uncertain observations. Test obfuscated/fake ore data, unknown chunk and later authoritative replacement; no claim to reconstruct server-withheld ore. |
| PacketFly | Ordinary authorized flight/glide/navigation alternatives with actual ability checks. Test repeated corrections stop task, changed ability and mount state; do not transplant movement-status spoofing. |
| Phase | Read-only camera inspection or ordinary safe route around obstruction. Test real player collision remains intact, no valid route and unknown neighboring cell. |
| PingSpoof | Show actual measured round-trip jitter/latency and sample age; local test simulations stay separate from live state. Test clock discontinuity, no recent samples and delayed response identity. |
| PortalGodMode | Portal transition status and ordinary danger warning, preserving actual damage mechanics. Test failed portal, incoming damage and dimension swap invalidating pending actions. |
| Reach | Render true native reach/hitbox debug and target eligibility; never enlarge accepted action range by visual setting. Test preview intersects enlarged box but real ray misses and block/entity competition. |
| Swing | Local swing visibility/animation policy with normal host networking semantics. Test offhand/main-hand, cancelled interaction, arm animation disabled and actual hit still recorded. |

## Movement

| Feature | Original native design and individual acceptance requirement |
|---|---|
| AntiLevitation | Render comfort/effect timer plus ordinary lateral movement and ceiling-risk warning. Test effect expiry, gravity change and levitation server update; visual override must not remove authoritative status. |
| AutoWalk | Owned ordinary movement request with manual input/menu/session cancellation and optional route. Test held back key, obstacle, edge, lag pause and no replay after long frame. |
| BoatFly | Ordinary boat navigation and telemetry; flight only when game grants an applicable mechanic. Test mount owner, dismount, collision, water boundary and accepted vehicle position. |
| ElytraFly | Ordinary takeoff/pitch/rocket controller with actual glider/durability/clearance and correction stop. Test water, levitation, no rocket, unknown corridor and manual camera control. |
| EntityControl | Route actual steering/jump input only through a controllable native mount and required equipment. Test untamed/no-saddle custom capability, passenger slot and vehicle unload. |
| EntitySpeed | Measure actual accepted mount speed and use native ordinary sprint/boost capability. Test speed potion/custom attribute, turn friction and correction; no arbitrary accepted multiplier promise. |
| FakeLag | Local motion-history/latency visualization with clear observed versus estimated positions. Test reconnect, stale snapshots, entity teleport and no misleading remote visibility claim. |
| FastAccel | Ordinary movement responsiveness and acceleration HUD can expose actual physics. Test diagonal input, ice, grounded/air transition and effect modifiers; do not silently edit host friction. |
| FastFall | Native swept landing preview and safe ordinary descent controls. Test stairs, hole edge, fluid and accepted fall damage; altered gravity remains separate. |
| FastSwim | Ordinary swim/sprint/pose intent under native abilities and water physics. Test surface transition, dolphin-style effect, flowing water and low ceiling. |
| Flight | Respect received creative/spectator/native flight flags with safe input cleanup. Test flight revoked midair, reconnect and changed fly speed. |
| HighJump | Native jump-strength/effect preview with ordinary grounded request. Test jump boost, custom gravity, ceiling and no granted height increase. |
| IceSpeed | Terrain route cost and actual friction telemetry; optional local test overrides need separate scope. Test mixed ice/ordinary boundary, fluid above ice and restored fact revision. |
| Jesus | Normal swim/boat route as practical liquid travel, preserving actual collision. Test source/flowing fluid, waterlogged block and no phantom solid support. |
| LongJump | Bounded normal jump-range/landing simulation and ordinary sprint-jump timing. Test narrow platform, inherited motion, ceiling and unknown landing. |
| NoFall | Ordinary reachable clutch proposal and fall-risk preview, with acknowledged landing before collecting water. Test evaporating water, late use, partial landing and no valid support. |
| NoJumpDelay | Grounded jump-request scheduler respects native cooldown and real input state. Test hold beneath low ceiling, landing tick, effect changes and manual release. |
| NoSlow | Separate ordinary use/terrain controls from authoritative movement factors. Test food/shield/bow, web/water, custom components and server slowdown. |
| Parkour | Next-footprint edge detection plus grounded normal jump and headroom check. Test diagonal ledge, sneak, fence/slab and no safe landing. |
| Speed | Accepted velocity HUD and normal sprint/jump controller; no source-specific spoofed speed mode credited. Test potion, collision, low TPS and native correction. |
| Sprint | Ordinary sprint state lease with hunger/use/direction eligibility and manual override. Test physical sprint through disable, backward motion and shield use. |
| Step | Use native granted step attributes or ordinary jump/navigation to a raised cell. Test stair/fence/full block, low ceiling and fluid transition. |
| TickShift | Bound task deadlines without replaying deferred action bursts; diagnose simulation lag separately from render lag. Test long pause and task cancellation before resumed tick. |
| TridentFly | Ordinary riptide eligibility, weather/fluid checks and native launch preview. Test no water/rain, enchantment absent, durability and glider transition. |
| Velocity | Received impulse vector/knockback HUD and ordinary motion response. Test explosion/melee ordering, fluid interaction and correction; suppression is not assumed authorized physics. |
| Yaw | Nearest-cardinal normal view controller with angular wrap, sensitivity and rotation ownership. Test exactly halfway, ±180, active bow and manual override. |

## Render

| Feature | Original native design and individual acceptance requirement |
|---|---|
| BetterChat | Structured message history with timestamp/dedup/filter options, sender identity and bounded bytes. Test same text/different sender, resize wrapping, invalid regex and preserved signed metadata. |
| BlockHighlight | Exact native selection shape and targeted storage marker from current ray. Test stairs/fence, fluid, player inside shape and empty/unknown target. |
| BreadCrumbs | Camera-relative bounded position ring and teleport/session strip breaks. Test far negative coordinates, reconnect same dimension, long stationary period and cap. |
| ESP | Shared received-entity snapshot, interpolated real bounds and category/relation policy. Test near plane, sleeping/custom pose, unknown health and dense cap. |
| ExtraTab | Received player list extensions with layout/cache budgets and explicit unknown metadata. Test long Unicode name, resize, simultaneous join/leave and missing game mode. |
| FOVModifier | Finite local FOV override with input/camera ownership and restoration to latest setting. Test zoom/sprint overlap, very small window and disable mid-animation. |
| Freecam | Independent local camera, normal body simulation and owned movement/look inputs. Test body damage, world leave, physical keys held and camera/body coordinate HUD distinction. |
| Fullbright | Original render-only brightness policy preserving actual world light and incoming effects. Test Xray overlap, night vision, reload and latest-state restoration. |
| InventoryInfo | Component-aware shulker/book preview and bounded recursive contents/tooltip layout. Test unknown contents, nested large component payload, custom enchantment and search persistence. |
| NameProtect | Local display alias layer retaining true identity in internal relations/actions. Test self/friend/other names, substring collision, chat copy/export and no network identity change claim. |
| Nametags | Stable projected received health/items/relation data with distance/culling budget. Test unknown values, item glint/name, overlapping players and first/third-person view. |
| NoBob | Local camera/hand bob amplitude control, preserving movement simulation and accessibility ownership. Test sprint/landing, user preference change while active and disable. |
| NoRender | Per-effect/local class render policy with bounded disabled cost and alternate critical state indicators. Test fog/particles/entity layers independently and correct pipeline restore. |
| NoRotate | Optional local camera smoothing independent of actual received action/body orientation. Test correction during use, vehicle and another rotation owner. |
| NoWeather | Visual rain/thunder policy retaining authoritative latest weather. Test incoming storm transition, dimension change and disable restoration. |
| OldPotions | Original legacy-inspired palette/glint option keyed by actual effects/components, no reference assets. Test custom potion, mixed effects, resource reload and color accessibility. |
| Search | Incremental received-block index with per-frame work/result cap and stale-job rejection. Test filter changed midscan, block replacement, unknown neighbor and limit indicator. |
| Shader | Original bounded postprocess/material pass, clearly distinguishing outline/glow/fill capabilities. Test resize, alpha/depth, resource loss and disabled GPU allocation. |
| Skeleton | Received model-pose bone segments transformed by actual animation/camera matrices. Test crouch/swim/glide, custom model missing bones and far-coordinate precision. |
| Skybox | Original local sky material/time/color settings with real weather available separately. Test no-sky dimension, resource reload and restored rendering state. |
| SmallShield | Local offhand transform override retaining actual shield use/bounds. Test main-hand/offhand, blocking pose, map and animation interruption. |
| Tracers | Camera-correct clipped lines to shared received target positions and finite primitive cap. Test directly behind/inside camera, unknown relation and screen resize. |
| Trajectories | Native read-only shadow projectile tick and actual collision/fluid queries with global segment/target budget. Test charge, source movement, water, unknown corridor and first-segment collision. |
| ViewClip | Local third-person distance/collision controls with finite clamps and perspective restore. Test inside wall, near plane, scroll ownership and freecam overlap. |
| ViewModel | Original copied first-person matrices per hand, finite rotation/translation/scale and cleanup. Test bow/map/food, zero/negative scale and resource reload. |
| Waypoints | World/server/dimension-scoped durable positions, actual distance labels and ordinary selected-target walking. Test dimension mismatch, body/freecam origin, duplicate name and import validation. |

## World

| Feature | Original native design and individual acceptance requirement |
|---|---|
| AirPlace | Preview whether actual native placement supports a target; ordinary executor requires accepted support/face rules. Test unsupported air cell, custom placement item and server rejection. |
| AntiInteract | User-configured ordinary block/entity interaction filter using exact IDs and hand policy. Test sneaking, offhand, use-item fallback and excluded container versus adjacent block. |
| AutoMine | Target-state mining planner with actual tool/LOS/reach and serial normal break acknowledgment. Test replaced block, custom hardness, player movement and active food. |
| AutoTunnel | Bounded oriented tunnel cross-section plan plus native navigation/mining under explicit region/material rules. Test liquid opening, gravity block, protected ore, unknown frontier and user cancellation. |
| Avoid | Shape-aware normal route/movement hazard cost with explicit categories and unknown handling. Test cactus side, magma top, berry/turtle-egg state and pressure plate. |
| FastPlace | Normal repeated placement input under actual item cooldown and acknowledgment; preview capability separate. Test bucket versus block versus food and duplicate event dispatch. |
| Fucker | Explicit selected-block removal queue with ordinary mining rules, independent from broader Nuker volume. Test valuable lookalike, allied/protected objective and unreachable block. |
| GhostHand | Received occluded target preview plus true native permitted action check. Test foreground entity/container, wall and beyond-reach object; don't synthesize hidden target. |
| MultiTask | Shared action arbitration permits only native-compatible attack/use combinations. Test shield/food/mining, offhand/mainhand conflict and a manual action winning. |
| NoGlitchBlocks | Track predicted versus acknowledged block revisions and expose pending/rejected state without local credit. Test reordered/late updates, dimension swap and rejected break/placement. |
| Nuker | Bounded shape/filter scan and normal one-block mining queue with tool lease. Test unbreakable, unknown, protected block, target cap and user target change. |
| Scaffold | Supported below-player placement with actual collision/reach/material policy and acknowledged progress. Test diagonal edge, container support, falling block and no item. |
| Speedmine | Native break-time estimate and tool optimization within actual mechanics. Test underwater/airborne penalties, haste/fatigue, wrong tool and rejected instant rebreak. |
| Wallhack | Original received-block transparency filter with correct render passes and bounded revisioned rebuild. Test fluids/translucency, exposure neighbors, filter changed mid-rebuild and restore. |

## Miscellaneous

| Feature | Original native design and individual acceptance requirement |
|---|---|
| Announcer | Local event summary and prepared message preview; network announcements require explicit configured send automation. Test repeated event dedup, coordinate privacy, rate limit and disconnect cancellation. |
| AntiAFK | Optional normal movement/look/swing activity under shared ownership and safe location checks. Test edge, busy hand, manual input and server idle policy; no guaranteed exemption. |
| AntiAim | Local cosmetic avatar orientation separate from actual camera/body/action targeting. Test use/attack alignment, vehicle and perspective. |
| AntiBookBan | Bounded decoder diagnostics/recovery rather than unlimited payload allocation; show possible unsynchronized inventory. Test compressed expansion, depth/entry budget and successful reconnect/resync. |
| AntiSpam | Bounded compiled regex/message filter with per-pattern validation and retained message identity. Test adjacent invalid regexes, costly expression, emoji and system messages. |
| AntiVanish | Last-observed player-list/entity events with timestamp and uncertain cause. Test chunk unload, hidden server data and renamed identity; never infer undisclosed coordinates. |
| AutoAccept | Parse only explicit supported teleport-request format from verified friend identity, with expiry and user-configured ordinary command. Test spoofed displayed text, stale request, renamed friend and disabled automation. |
| AutoEat | Component-safe food eligibility and hunger/health policy, normal use lease and finish-current-bite. Test harmful/valuable food, full hunger/low health, manual slot override and modified stack midbite. |
| AutoFish | Owner bobber state machine, received bite signal and bounded cast/reel/recast deadlines. Test unrelated splash, no rod, hook unloaded, full inventory and remote latency. |
| AutoMount | Eligible received mount ranking, ordinary interaction and cooldown/owner checks. Test unsteerable mount, passenger occupied, friend vehicle, target unload and failed interact. |
| AutoReconnect | Normal connection retry with monotonic countdown/backoff, exact saved endpoint and cancel. Test intentional leave, account handoff, address change and failed login. |
| AutoRespawn | Ordinary delayed respawn with session/death generation and death mark capture. Test duplicate death event, user cancels, world leave and changed dimension. |
| BeaconSelector | Effect requirement reference plus ordinary valid beacon editor; extra button choices do not grant an illegal effect. Test level/payment/server rejection and changed menu. |
| ChestAura | Ordinary reachable-container discovery/open/transfer requires explicit region/permission and exact menu ownership. Test blocked chest, another screen, inventory full, duplicate open and private contents not received. |
| ChestStealer | Open-container normal transfer queue with filter, cursor precondition and one acknowledged move. Test merchant/crafting output, replaced menu, full destination and user click during delay. |
| Heaven | Local death-camera effect can be original and purely visual; actual dead player state remains unchanged. Test respawn interrupts animation, world leave and perspective restore. |
| InvCleaner | Preview configurable junk/dominated equipment before ordinary drop queue with protected slots/components. Test enchanted/custom-named item, cursor, manual slot change and full replacement policy. |
| MiddleClick | Current true ray target chooses local friend or explicitly configured normal action, debounced and screen-aware. Test mob/item/no target, offhand use and GUI middle click. |
| NoSoundLag | Coalesce/rate-limit selected local audio events and bound active voices. Test exploit-like burst, looping sound, important cue exclusion and disable. |
| NoteBot | Bounded decoded-song timeline and local preview first; ordinary reachable-block playback is a separate action mode. Test missing note/instrument, empty file, lagged tune and pause/resume. |
| NoteTuner | Native note-state target with modulo pitch-cycle count, ordinary use and received-state recheck. Test block edited by another player, delayed update, unreachable block and bounded retries. |
| SkinBlink | Layer-mask animation with independent user layer ownership and bounded interval. Test cape absent, layer preference changed while active and reduced motion. |
| Sneak | Ordinary sneak intent with physical-key ownership, actual body state and edge behavior. Test manual sneak through disable, riding, food and focus. |
| SongStealer | Record only received note events with bounded timestamp/instrument track and explicit local save. Test unrelated sounds, unavailable instrument metadata, event burst and session switch. |
| Spammer | A prepared user-invoked message queue can split Unicode safely with strict rate/cancel; sending requires explicit configured automation. Test stale split continuation, command prefix, disconnect and emoji boundary. |
| Timer | Keep actual native simulation rate authoritative; local UI timing/diagnostics remain separate. Test long frame, low TPS, pause/resume and no accumulated action burst. |
| UnfocusedFPS | Native focus-aware frame cap with immediate focus restoration and minimum background responsiveness. Test minimized/focused transitions, overlay focus, uncapped setting and no background busy loop. |
| XCarry | Ordinary safe inventory layout/crafting preview, preserving server close semantics and carried stack. Test close menu with cursor, rejected transaction and world leave; no extra server storage promise. |

## Other

| Feature | Original native design and individual acceptance requirement |
|---|---|
| Capes | Original local cosmetic assets or explicitly supported identity service, distinct from others' received cosmetics. Test missing cape, cache/resource reload and no unauthorized identity/network claim. |
| Chat | Native ordinary chat UI/send with explicit recipient/context and bounded history; any external client network is a separate integration. Test offline service, Unicode, rate limit and unsent draft preservation. |
| ClickGui | Native typed searchable settings, binding capture, scale and focus ownership with original layout. Test tiny window, text input versus hotkey, hold release inside menu and profile reload. |
| Colors | Central validated palette/tokens, retained alpha and accessible contrast with deterministic theme revision. Test RGB/HSV/hex round-trip, alpha zero, invalid input and active animation. |
| Font | Original/bundled permitted font stack, bounded glyph atlas and Unicode fallback. Test emoji/CJK, missing font, high DPI and cache invalidation. |
| HUD | Independent typed observation providers, layout persistence, units and unknown status; bounded text/geometry budgets. Test resolution change, absent world/player, long strings and refresh interval. |
| Rotations | One shared ordinary rotation arbiter with target/session/priority/expiry, sensitivity-correct output and manual-input cancellation. Test concurrent bow/build/food request, wrap, pitch clamp and world leave. |
| Baritone | Reuse Horizon's bounded ordinary path tasks with partial/failure reporting; no claim to implement an external mod. Test unreachable goal, unknown corridor, stale task ID, dynamic obstacle and manual cancel. |

The current source-of-truth scope is the seven observed public panels. All module implementation/architecture claims beyond those public contracts remain unknown. Proposed native substitutes intentionally preserve useful outcomes through ordinary host actions and received information; a substitute is individually named and tested rather than silently called identical feature parity.
