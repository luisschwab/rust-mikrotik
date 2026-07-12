# Network Crawling Process

This document describes how `mikrotik-crawler` crawls a RouterOS network,
turns raw RouterOS API state into a topology graph, and renders that graph into
inspection artifacts.

The crawler is read-only. It does not create users, enable services, change
routes, or provision devices. Every graph relationship is derived from collected
RouterOS state or explicitly retained discovery/failure evidence.

## Inputs

A crawl starts from one or more `DeviceTarget` values:

- `address`: connectable RouterOS API target, usually `host:port`.
- `credentials`: username and optional password.

The CLI exposes those as:

```text
cargo run -p mikrotik-crawler --bin crawler -- \
  --seed 10.20.20.1:8728 \
  --user read-only \
  --password '...' \
  --max-depth 4 \
  --connect-timeout-seconds 8 \
  --connect-retries 2
```

Additional owned devices can be supplied with repeated `--seed` flags. These are
treated as extra owned seeds, which is useful for owned BGP routers that may not
be reachable through neighbor recursion from the primary seed.

The crawler also accepts a `TargetResolver`. The live CLI normally uses the
discovered IP address directly. Tests and simulated networks can use a static
mapping so an address discovered in RouterOS state, such as `198.51.100.1`, is
resolved to a connectable host-forward target, such as `127.0.0.1:54036`.

## Crawl Queue

The crawler maintains a queue of targets and a set of known target addresses.
Each queued target has:

- the API address to connect to;
- credentials to use;
- crawl depth;
- neighbor evidence that led to it, when applicable.

Seed targets are depth `0`. Neighbor-discovered targets are queued at
`parent_depth + 1`.

`max_depth` limits API collection, not just rendering. When a device is
collected at the maximum depth, the crawler records its neighbor rows but does
not queue those neighbors for collection. This keeps large live networks from
expanding past the operator-requested boundary.

The crawler also enforces:

- `max_devices`: maximum number of successfully collected devices;
- `max_concurrency`: maximum concurrent snapshot jobs;
- `address_family`: usually IPv4 by default, so IPv6 link-local neighbors are
  skipped for recursive collection unless explicitly allowed;
- `connect_timeout` and `connect_retries`: short API failures are retried before
  becoming failed neighbor evidence.

## Snapshot Collection

For each target selected from the queue, the crawler connects through
`mikrotik-client` using `api-ssl` by default and falling back to plaintext
`api` when the TLS service cannot be reached. Pass `--protocol api` to force
plaintext-only collection.

On successful connection it collects a typed `DeviceSnapshot` from RouterOS API
print commands. The snapshot includes, when supported by the RouterOS version:

- `/system/identity/print`
- `/system/resource/print`
- `/system/routerboard/print`
- `/interface/print`
- `/ip/address/print`
- `/ip/neighbor/print`
- `/ip/route/print`
- BGP state:
  - RouterOS v7 connection/session tables;
  - RouterOS v6 peer state where available.

Older RouterOS versions may return a trap for commands that do not exist. Those
command-specific traps are treated as unsupported command results, not as a
device collection failure. The snapshot simply has an empty vector for that
unsupported state.

Each snapshot stores the target address used to collect it. That target address
is later used as the node management address in tooltips.

## Neighbor Discovery

`/ip/neighbor` is used for discovery only.

Neighbor rows are not treated as topology edges by themselves. They are used to
find more MikroTik devices that may be worth collecting.

For each neighbor row, the crawler extracts:

- neighbor identity;
- neighbor management address;
- local interface where the neighbor was seen;
- remote interface when RouterOS reports it;
- platform, board, version, and MAC evidence when present.

The neighbor is eligible for recursive collection when:

- it has a usable management address;
- it matches the configured address-family filter;
- it is not already known or queued;
- `max_depth` has not been reached.

When eligible, the crawler resolves the discovered address through the
configured `TargetResolver`, keeps the neighbor evidence, and queues the target
with the same credentials used for the crawl.

## Failure Handling

Seed target failures are fatal. If the crawler cannot collect the primary seed
or an explicitly provided seed, it returns an error immediately because there is
no trustworthy topology root.

Neighbor target failures are not fatal. They are recorded and carried into graph
construction as failed neighbor evidence.

Failure reasons currently affect rendering like this:

- bad credentials: inferred node with dashed red outline and
  `ERROR: BAD CREDENTIALS`;
- API connection refused: inferred node with dashed outline and
  `ERROR: API DISABLED`;
- timeout or route/connect failure: inferred node with dashed outline and
  `ERROR: UNREACHABLE`.

These nodes remain visible when the crawler has enough neighbor or L3 evidence
to place them. This is intentional: an inaccessible device is operationally
important and should not disappear from the map.

## Graph Nodes

After crawling, `build_graph_with_neighbor_evidence` creates a `NetworkGraph`.

Each RouterOS device is one `NetworkNode`.

Collected nodes are built from `DeviceSnapshot::stable_key()`, usually based on
stable device identity such as serial number. Collected nodes keep the full
`DeviceSnapshot`.

Inferred nodes are built from neighbor evidence when the crawler saw a device
but could not collect it, or when an opaque external BGP peer is known only by
remote BGP address. Inferred nodes keep only the evidence available from the
owning device, such as identity, management address, board/version, MAC address,
and failure reason.

The graph never invents state for a device it did not collect. For example, an
external upstream BGP peer can appear as a node, but it has no remote interface
name because that information is not available from our routers.

## Address Indexes

Graph construction builds helper indexes from collected snapshots:

- target address to node key;
- target host without API port to node key;
- interface IP host address to `(node, interface)`;
- interface name to configured prefixes for rendering.

These indexes are used to connect route next-hops to devices and to render
link-specific address information later.

## Graph Edges

Edges are not built directly from `/ip/neighbor`. They come from routing and
interface state.

### L3 Interface Links

The graph scans `/ip/address` on every collected device and normalizes each
interface prefix into an `L3Network`.

Only small point-to-point style networks are considered topology links:

- IPv4 `/29`, `/30`, and `/31`;
- IPv6 `/126` and `/127`.

Bridge interfaces are skipped for L3 topology edges. Broad management networks
such as `10.100.0.0/24` are also skipped, because they describe management
reachability, not physical or routed topology.

When two different devices have addresses in the same small normalized network,
the graph creates one L3 edge between those device interfaces.

### Route Next-Hop Links

The graph scans active route entries.

A route contributes topology evidence only when:

- `active=yes`;
- `disabled` is not true;
- `inactive` is not true;
- it is not a connected route.

The next-hop is parsed from `immediate-gw` first and `gateway` second. Scoped
gateways such as `10.0.0.1%ether2` preserve the scoped interface when present.

If the next-hop address maps to a collected device address, the graph creates a
route edge from the local device to that remote device. The local interface is
chosen from the scoped gateway, route VRF interface, or a local prefix that
contains the next-hop.

This is how the graph represents routed relationships that are not visible as a
direct shared `/30` or `/29`.

### BGP Links

BGP edges come from collected BGP state, not from neighbor discovery.

For owned routers, the crawler reads BGP sessions/connections/peers where the
RouterOS version supports those commands. Disabled BGP entries are ignored.
Established sessions get higher confidence, but configured active peers can
still be represented when they are enabled and known.

When the remote BGP address belongs to another collected owned router, the edge
connects those two collected nodes.

When the remote BGP address does not map to a collected node, the graph creates
an opaque inferred BGP node. This is the expected representation for external
upstream routers, because we do not control them and cannot know their
interfaces.

### Fallback Links

Fallback edges are only used after graph construction sees a collected or failed
node that would otherwise float disconnected.

The fallback uses retained neighbor evidence to anchor that node back to the
device that discovered it. This is weaker than L3 or route evidence, and is
classified as `fallback`, not as a real topology relationship.

This exists so operationally important nodes with bad credentials, disabled API,
or partial configuration do not vanish or float disconnected while the network
is being fixed.

## Edge Collapsing and Classification

The graph can contain reciprocal directed evidence. Before Graphviz export, the
renderer collapses reciprocal edges into one visual edge.

The collapsed visual edge keeps the best known local and remote interfaces. BGP
evidence wins BGP classification for that device pair.

Visual link kinds are:

- `bgp`: BGP control-plane relationship;
- `route`: active route next-hop evidence;
- `internal`: core/internal relationship inferred from device roles;
- `customer`: customer-facing relationship inferred from device roles;
- `management`: broad management fallback relationship;
- `fallback`: neighbor-discovery anchor for otherwise disconnected nodes;
- `unknown`: no stronger classification available.

`LinkFilter` controls which link kinds are rendered. The CLI default is
`routing`, which excludes ordinary management links but keeps route, L3, BGP,
customer/internal, and fallback links.

## Link Address Rendering

Link labels and tooltips intentionally do not show every address on an
interface.

For each visual edge, the renderer builds both endpoint labels together so it
can decide which addresses are relevant to that specific link.

It includes:

- the interface prefix that is part of the same small L3 link network on both
  endpoints;
- the management address when it is distinct from the displayed link prefix;
- public prefixes on customer-facing links, so public customer allocations are
  visible.

It excludes:

- unrelated private aliases on the same interface;
- broad management network addresses that are not the edge being rendered;
- disabled or invalid address rows, because they were already excluded from L3
  edge discovery.

This keeps edge hover text focused on the addresses needed to understand that
relationship.

## Node Tooltip Rendering

Node tooltips are concise operational summaries.

Collected nodes show:

- status;
- identity/name;
- management address;
- serial number;
- RouterOS version;
- board;
- uptime;
- CPU load;
- memory usage;
- storage usage.

The tooltip intentionally omits architecture, CPU model/count/frequency, and
bad blocks to keep the hover content readable.

Inferred nodes show the evidence available from discovery, such as management
address, board/platform/version, MAC address, and failure reason.

## Export Pipeline

The CLI writes all artifacts into one run directory. If `--outdir` is omitted,
the directory is:

```text
mikrotik-crawler/src/bin/runs/live/<YYYYMMDD-HHMMSS>
```

After a successful run, `mikrotik-crawler/src/bin/runs/live/latest` is updated to
point at the newest run directory.

The primary Graphviz export is SFDP:

- `topology.dot`: DOT using `layout=sfdp`;
- `topology.svg`: Graphviz-rendered SVG;
- `topology.interactive.html`: the SVG wrapped with pan/zoom and custom
  tooltips;
- `topology.png`: optional, only when `--png` is passed.

SFDP is the default because it handles large live networks better than a single
horizontal rank row. The DOT still includes invisible semantic rank anchors so
the force layout is biased into top-to-bottom sections:

1. external BGP peers;
2. owned BGP routers;
3. border or edge routers;
4. core/OSPF/internal routers;
5. customer routers;
6. unknown or uncategorized nodes.

Radio/backhaul devices get one additional placement heuristic. If a collected
device name looks like `<src>-<dst>`, for example `Orbitel-QI23` or
`QI23-ESCmusica`, the renderer treats it as a point-to-point radio/backhaul hop
for placement purposes. This does not create topology edges. It only changes
rank anchoring: radio nodes, and nodes downstream from a radio hop, can use
their real downstream depth instead of being collapsed into the generic customer
section. That lets chains such as:

```text
Rt_Border -> Orbitel-QI23 -> QI23-ESCmusica -> Rt_ESCmusica
```

draw as a downstream chain instead of a flat customer row.

## Scenario Testing

The crawler can start a simulated network from the QEMU runner scenario corpus
with `--scenario`, wait until the scenario links and RouterOS APIs are ready,
run the crawler, and export the same topology artifacts under:

```text
mikrotik-crawler/src/bin/runs/scenario/<YYYYMMDD-HHMMSS>
```

The QEMU runner scenario is useful for validating:

- recursive discovery through `/ip/neighbor`;
- route and L3 edge construction;
- BGP edge construction;
- inaccessible-node rendering;
- customer public prefix rendering;
- Graphviz output shape.

Run it with:

```text
cargo run -p mikrotik-crawler --bin crawler -- \
  --run-kind scenario \
  --scenario mikrotik-qemu-runner/scenarios/isp-network.toml \
  --mode one-shot \
  --protocol api
```

## Live Debugging Checklist

When the rendered graph looks wrong, check these in order:

1. Confirm the crawler collected the expected routers. The CLI prints node and
   failure counts, and `crawl.log` contains per-target collection logs.
2. If a node is missing entirely, verify it appears in `/ip/neighbor` from a
   collected device or seed it explicitly with `--seed`.
3. If a node exists but is inferred, check its failure reason:
   bad credentials, API disabled, or unreachable.
4. If a collected node floats disconnected, check whether it has a small L3
   prefix, active route next-hop, BGP state, or retained neighbor fallback
   evidence.
5. If a customer appears connected through the management network, check that
   broad management prefixes are not being used as topology links. The graph
   should rely on small L3 prefixes, routes, BGP, or fallback anchors.
6. If public customer addresses are missing from a link tooltip, verify the
   link is classified as customer-facing and the public prefixes are configured
   on the relevant interface.
7. If a BGP peer is missing, verify the BGP entry is enabled and visible through
   the RouterOS command supported by that RouterOS version.
