use core::f64::consts::PI;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

use mikrotik_types::abstractions::LinkKind;
use mikrotik_types::device::TopologyNodeKey;

use super::GRAPHVIZ_RANK_CORE_OSPF;
use super::GRAPHVIZ_RANK_CUSTOMER;
use super::GRAPHVIZ_RANK_EDGE_BORDER;
use super::GRAPHVIZ_RANK_OWNED_BGP;
use super::GRAPHVIZ_RANK_UPSTREAM;
use super::edge::GraphvizEdge;
use super::graphviz_node_has_bgp_state;
use super::graphviz_rank;
use super::model::NetworkGraph;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_CHILD_SPAN;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_DISCONNECTED_OFFSET;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_DISCONNECTED_SPACING;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_MAX_ROW_CHILDREN;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_RADIUS;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_ROOT_END;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_ROOT_START;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_ROW_GAP;
use crate::constants::GRAPHVIZ_SECTION_BORDER_Y;
use crate::constants::GRAPHVIZ_SECTION_CORE_Y;
use crate::constants::GRAPHVIZ_SECTION_CUSTOMER_Y;
use crate::constants::GRAPHVIZ_SECTION_DEEP_DOWNSTREAM_GAP;
use crate::constants::GRAPHVIZ_SECTION_NODE_SPACING;
use crate::constants::GRAPHVIZ_SECTION_OWNED_BGP_Y;
use crate::constants::GRAPHVIZ_SECTION_ROW_GAP;
use crate::constants::GRAPHVIZ_SECTION_ROW_WIDTH;
use crate::constants::GRAPHVIZ_SECTION_UNKNOWN_Y;
use crate::constants::GRAPHVIZ_SECTION_UPSTREAM_Y;
use crate::constants::GRAPHVIZ_TYPED_RADIAL_CHILD_COLUMN_SPACING;
use crate::constants::GRAPHVIZ_TYPED_RADIAL_CHILD_ROW_OFFSET;
use crate::constants::GRAPHVIZ_TYPED_RADIAL_CHILD_ROW_SPACING;
use crate::constants::GRAPHVIZ_TYPED_RADIAL_MIN_ANGLE;
use crate::constants::GRAPHVIZ_TYPED_RADIAL_RING_DISTANCE;
use crate::options::DotExportOptions;

/// Return fixed polar coordinates for a typed radial topology map.
pub(super) fn typed_radial_positions(
    graph: &NetworkGraph,
    edges: &[GraphvizEdge],
    visible_nodes: &BTreeSet<TopologyNodeKey>,
    root_node: Option<&str>,
) -> BTreeMap<TopologyNodeKey, (f64, f64)> {
    let root = root_node
        .and_then(|root| visible_nodes.iter().find(|node| node.as_str() == root).cloned())
        .or_else(|| {
            graph
                .nodes
                .iter()
                .find(|node| visible_nodes.contains(&node.key))
                .map(|node| node.key.clone())
        });
    let Some(root) = root else {
        return BTreeMap::new();
    };

    let typed_nodes = typed_radial_tree(edges, visible_nodes, &root);
    let mut root_sectors = BTreeMap::<LinkKind, Vec<TopologyNodeKey>>::new();
    let mut children = BTreeMap::<TopologyNodeKey, Vec<TopologyNodeKey>>::new();
    let mut positions = BTreeMap::new();
    positions.insert(root.clone(), (0.0, 0.0));

    for node in visible_nodes {
        if *node == root {
            continue;
        }
        let placement = typed_nodes
            .get(node)
            .cloned()
            .unwrap_or_else(|| fallback_typed_radial_placement(node, edges, &typed_nodes));
        if placement.parent.as_ref() == Some(&root) {
            root_sectors.entry(placement.link_kind).or_default().push(node.clone());
        } else if let Some(parent) = placement.parent {
            children.entry(parent).or_default().push(node.clone());
        } else {
            root_sectors.entry(placement.link_kind).or_default().push(node.clone());
        }
    }

    let mut queue = VecDeque::new();
    for (link_kind, mut nodes) in root_sectors {
        nodes.sort();
        let (sector_start, sector_end) = typed_radial_sector(link_kind);
        let angles = typed_radial_angles(sector_start, sector_end, nodes.len());
        for (node, angle) in nodes.into_iter().zip(angles) {
            let depth = typed_nodes.get(&node).map_or(1, |placement| placement.depth.max(1));
            let radius = usize_to_f64(depth) * GRAPHVIZ_TYPED_RADIAL_RING_DISTANCE;
            positions.insert(node.clone(), (radius * angle.cos(), radius * angle.sin()));
            queue.push_back((node, angle));
        }
    }

    while let Some((parent, parent_angle)) = queue.pop_front() {
        let Some(mut parent_children) = children.remove(&parent) else {
            continue;
        };
        parent_children.sort();
        let parent_position = positions.get(&parent).copied().unwrap_or((
            GRAPHVIZ_TYPED_RADIAL_RING_DISTANCE * parent_angle.cos(),
            GRAPHVIZ_TYPED_RADIAL_RING_DISTANCE * parent_angle.sin(),
        ));
        let offsets = typed_radial_child_offsets(parent_children.len());
        for (node, (tangent_offset, outward_offset)) in parent_children.into_iter().zip(offsets) {
            let position = typed_radial_child_position(parent_position, parent_angle, tangent_offset, outward_offset);
            positions.insert(node.clone(), position);
            queue.push_back((node, parent_angle));
        }
    }

    positions
}

/// One typed-radial BFS placement.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TypedRadialPlacement {
    /// BFS depth from the root.
    depth: usize,
    /// Link kind that reached this node.
    link_kind: LinkKind,
    /// Parent node in the BFS tree.
    parent: Option<TopologyNodeKey>,
}

/// Return BFS tree placement data for nodes reachable from the root.
fn typed_radial_tree(
    edges: &[GraphvizEdge],
    visible_nodes: &BTreeSet<TopologyNodeKey>,
    root: &TopologyNodeKey,
) -> BTreeMap<TopologyNodeKey, TypedRadialPlacement> {
    let mut adjacency = BTreeMap::<TopologyNodeKey, Vec<(TopologyNodeKey, LinkKind)>>::new();
    for edge in edges {
        if !visible_nodes.contains(&edge.local_node) || !visible_nodes.contains(&edge.remote_node) {
            continue;
        }
        adjacency
            .entry(edge.local_node.clone())
            .or_default()
            .push((edge.remote_node.clone(), edge.link_kind));
        adjacency
            .entry(edge.remote_node.clone())
            .or_default()
            .push((edge.local_node.clone(), edge.link_kind));
    }
    for neighbors in adjacency.values_mut() {
        neighbors.sort();
    }

    let mut placements = BTreeMap::new();
    let mut queue = VecDeque::from([(root.clone(), 0_usize, LinkKind::Unknown, None)]);
    while let Some((node, depth, link_kind, parent)) = queue.pop_front() {
        if placements.contains_key(&node) {
            continue;
        }
        placements.insert(
            node.clone(),
            TypedRadialPlacement {
                depth,
                link_kind,
                parent,
            },
        );
        let Some(neighbors) = adjacency.get(&node) else {
            continue;
        };
        for (neighbor, neighbor_link_kind) in neighbors {
            if !placements.contains_key(neighbor) {
                queue.push_back((neighbor.clone(), depth + 1, *neighbor_link_kind, Some(node.clone())));
            }
        }
    }
    placements
}

/// Return a fallback placement for disconnected visible nodes.
fn fallback_typed_radial_placement(
    node: &TopologyNodeKey,
    edges: &[GraphvizEdge],
    placements: &BTreeMap<TopologyNodeKey, TypedRadialPlacement>,
) -> TypedRadialPlacement {
    let max_depth = placements.values().map(|placement| placement.depth).max().unwrap_or(0);
    let link_kind = edges
        .iter()
        .find(|edge| edge.local_node == *node || edge.remote_node == *node)
        .map_or(LinkKind::Unknown, |edge| edge.link_kind);
    TypedRadialPlacement {
        depth: max_depth + 1,
        link_kind,
        parent: None,
    }
}

/// Return one angular sector for a link kind.
fn typed_radial_sector(link_kind: LinkKind) -> (f64, f64) {
    match link_kind {
        LinkKind::Bgp => (PI * 0.58, PI * 0.95),
        LinkKind::Route => (PI * 0.96, PI * 1.04),
        LinkKind::Internal => (PI * 0.08, PI * 0.45),
        LinkKind::Customer => (PI * 1.05, PI * 1.45),
        LinkKind::Wireless => (PI * 1.46, PI * 1.54),
        LinkKind::Management | LinkKind::Fallback => (PI * 1.55, PI * 1.92),
        LinkKind::Unknown => (PI * 1.93, PI * 2.07),
    }
}

/// Convert a layout count to `f64` after bounding it to the exact integer range used by this renderer.
fn usize_to_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

/// Return the ceiling of the square root of a positive integer.
fn integer_sqrt_ceil(value: usize) -> usize {
    let mut root = 1usize;
    while root.saturating_mul(root) < value {
        root += 1;
    }
    root
}

/// Return evenly-spaced angles inside one sector.
fn typed_radial_angles(start: f64, end: f64, count: usize) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![(start + end) / 2.0];
    }
    let span = (end - start)
        .abs()
        .max(GRAPHVIZ_TYPED_RADIAL_MIN_ANGLE * usize_to_f64(count));
    let center = (start + end) / 2.0;
    let adjusted_start = center - span / 2.0;
    let step = span / usize_to_f64(count + 1);
    (0..count)
        .map(|index| adjusted_start + step * usize_to_f64(index + 1))
        .collect()
}

/// Return local child offsets clustered beyond one parent.
fn typed_radial_child_offsets(count: usize) -> Vec<(f64, f64)> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![(0.0, GRAPHVIZ_TYPED_RADIAL_CHILD_ROW_OFFSET)];
    }
    let columns = integer_sqrt_ceil(count).max(2);
    (0..count)
        .map(|index| {
            let row = index / columns;
            let column = index % columns;
            let row_len = (count - row * columns).min(columns);
            let tangent_offset = (usize_to_f64(column) - usize_to_f64(row_len.saturating_sub(1)) / 2.0)
                * GRAPHVIZ_TYPED_RADIAL_CHILD_COLUMN_SPACING;
            let outward_offset =
                GRAPHVIZ_TYPED_RADIAL_CHILD_ROW_OFFSET + usize_to_f64(row) * GRAPHVIZ_TYPED_RADIAL_CHILD_ROW_SPACING;
            (tangent_offset, outward_offset)
        })
        .collect()
}

/// Return one child position from local tangent/outward offsets.
fn typed_radial_child_position(
    parent_position: (f64, f64),
    parent_angle: f64,
    tangent_offset: f64,
    outward_offset: f64,
) -> (f64, f64) {
    let outward = (parent_angle.cos(), parent_angle.sin());
    let tangent = (-parent_angle.sin(), parent_angle.cos());
    (
        parent_position.0 + outward.0 * outward_offset + tangent.0 * tangent_offset,
        parent_position.1 + outward.1 * outward_offset + tangent.1 * tangent_offset,
    )
}

/// Return fixed coordinates for a recursive radial hierarchy.
pub(super) fn recursive_radial_positions(
    graph: &NetworkGraph,
    edges: &[GraphvizEdge],
    visible_nodes: &BTreeSet<TopologyNodeKey>,
    options: &DotExportOptions,
) -> BTreeMap<TopologyNodeKey, (f64, f64)> {
    let root = options
        .root_node
        .as_deref()
        .and_then(|root| visible_nodes.iter().find(|node| node.as_str() == root).cloned())
        .or_else(|| visible_nodes.iter().next().cloned());
    let Some(root) = root else {
        return BTreeMap::new();
    };

    let mut positions = BTreeMap::from([(root.clone(), (0.0, 0.0))]);
    let owned_bgp = recursive_owned_bgp_nodes(graph, visible_nodes, options);
    let external_bgp = visible_nodes
        .iter()
        .filter(|node| node.as_str().starts_with("bgp:"))
        .cloned()
        .collect::<Vec<_>>();

    recursive_place_arc_rows(
        &mut positions,
        (0.0, 0.0),
        PI * 0.16,
        PI * 0.84,
        GRAPHVIZ_RECURSIVE_RADIAL_RADIUS,
        &owned_bgp,
    );
    recursive_place_arc_rows(
        &mut positions,
        (0.0, 0.0),
        PI * 0.10,
        PI * 0.90,
        GRAPHVIZ_RECURSIVE_RADIAL_RADIUS * 2.0,
        &external_bgp,
    );

    let adjacency = recursive_downstream_adjacency(edges, visible_nodes, options);
    let children = recursive_radial_children(&root, &adjacency);
    let mut queue = VecDeque::from([(root.clone(), PI * 1.5)]);
    while let Some((parent, parent_angle)) = queue.pop_front() {
        let Some(parent_children) = children.get(&parent) else {
            continue;
        };
        let Some(parent_position) = positions.get(&parent).copied() else {
            continue;
        };
        let (start, end) = if parent == root {
            (GRAPHVIZ_RECURSIVE_RADIAL_ROOT_START, GRAPHVIZ_RECURSIVE_RADIAL_ROOT_END)
        } else {
            (
                parent_angle - GRAPHVIZ_RECURSIVE_RADIAL_CHILD_SPAN / 2.0,
                parent_angle + GRAPHVIZ_RECURSIVE_RADIAL_CHILD_SPAN / 2.0,
            )
        };
        let child_angles = recursive_place_arc_rows(
            &mut positions,
            parent_position,
            start,
            end,
            GRAPHVIZ_RECURSIVE_RADIAL_RADIUS,
            parent_children,
        );
        for (child, child_angle) in parent_children.iter().cloned().zip(child_angles) {
            queue.push_back((child, child_angle));
        }
    }

    let mut disconnected = visible_nodes
        .iter()
        .filter(|node| !positions.contains_key(*node))
        .cloned()
        .collect::<Vec<_>>();
    disconnected.sort();
    recursive_place_disconnected_rows(&mut positions, &disconnected);
    recursive_apply_horizontal_sections(&mut positions, graph, visible_nodes, options);

    positions
}

/// Align recursive radial nodes into top-to-bottom semantic horizontal sections.
fn recursive_apply_horizontal_sections(
    positions: &mut BTreeMap<TopologyNodeKey, (f64, f64)>,
    graph: &NetworkGraph,
    visible_nodes: &BTreeSet<TopologyNodeKey>,
    options: &DotExportOptions,
) {
    let mut sections = BTreeMap::<u8, Vec<TopologyNodeKey>>::new();
    for node in visible_nodes {
        if positions.contains_key(node) {
            sections
                .entry(graphviz_rank(node, graph, options))
                .or_default()
                .push(node.clone());
        }
    }

    for (rank, nodes) in sections {
        recursive_place_section_rows(positions, rank, nodes);
    }
}

/// Place one semantic section as one or more horizontal rows while preserving topology-derived X positions.
fn recursive_place_section_rows(
    positions: &mut BTreeMap<TopologyNodeKey, (f64, f64)>,
    rank: u8,
    mut nodes: Vec<TopologyNodeKey>,
) {
    nodes.sort_by(|left, right| {
        let left_position = positions.get(left).copied().unwrap_or_default();
        let right_position = positions.get(right).copied().unwrap_or_default();
        left_position
            .0
            .total_cmp(&right_position.0)
            .then_with(|| left.as_str().cmp(right.as_str()))
    });

    let mut rows = Vec::<Vec<TopologyNodeKey>>::new();
    for node in nodes {
        let x = positions.get(&node).map_or(0.0, |position| position.0);
        let should_start_row = rows.last().is_some_and(|row| {
            let first_x = row
                .first()
                .and_then(|node| positions.get(node))
                .map_or(x, |position| position.0);
            (x - first_x).abs() > GRAPHVIZ_SECTION_ROW_WIDTH
        });
        if should_start_row || rows.is_empty() {
            rows.push(Vec::new());
        }
        if let Some(row) = rows.last_mut() {
            row.push(node);
        }
    }

    for (row_index, row) in rows.into_iter().enumerate() {
        let y = recursive_section_y(rank) - usize_to_f64(row_index) * GRAPHVIZ_SECTION_ROW_GAP;
        let xs = recursive_spaced_section_x_positions(positions, &row);
        for (node, x) in row.into_iter().zip(xs) {
            positions.insert(node, (x, y));
        }
    }
}

/// Return row X positions with at least the configured minimum spacing.
fn recursive_spaced_section_x_positions(
    positions: &BTreeMap<TopologyNodeKey, (f64, f64)>,
    row: &[TopologyNodeKey],
) -> Vec<f64> {
    let mut xs = row
        .iter()
        .map(|node| positions.get(node).map_or(0.0, |position| position.0))
        .collect::<Vec<_>>();
    if xs.is_empty() {
        return xs;
    }

    let original_center = (xs[0] + xs[xs.len() - 1]) / 2.0;
    for index in 1..xs.len() {
        let minimum = xs[index - 1] + GRAPHVIZ_SECTION_NODE_SPACING;
        if xs[index] < minimum {
            xs[index] = minimum;
        }
    }
    let adjusted_center = (xs[0] + xs[xs.len() - 1]) / 2.0;
    let delta = original_center - adjusted_center;
    for x in &mut xs {
        *x += delta;
    }
    xs
}

/// Return the baseline Y coordinate for one semantic section.
pub(super) fn recursive_section_y(rank: u8) -> f64 {
    match rank {
        GRAPHVIZ_RANK_UPSTREAM => GRAPHVIZ_SECTION_UPSTREAM_Y,
        GRAPHVIZ_RANK_OWNED_BGP => GRAPHVIZ_SECTION_OWNED_BGP_Y,
        GRAPHVIZ_RANK_EDGE_BORDER => GRAPHVIZ_SECTION_BORDER_Y,
        GRAPHVIZ_RANK_CORE_OSPF => GRAPHVIZ_SECTION_CORE_Y,
        GRAPHVIZ_RANK_CUSTOMER => GRAPHVIZ_SECTION_CUSTOMER_Y,
        _ => {
            if rank > GRAPHVIZ_RANK_CUSTOMER {
                GRAPHVIZ_SECTION_CUSTOMER_Y
                    - usize_to_f64(usize::from(rank - GRAPHVIZ_RANK_CUSTOMER)) * GRAPHVIZ_SECTION_DEEP_DOWNSTREAM_GAP
            } else {
                GRAPHVIZ_SECTION_UNKNOWN_Y
            }
        }
    }
}

/// Return owned BGP nodes from explicit seed context.
fn recursive_owned_bgp_nodes(
    graph: &NetworkGraph,
    visible_nodes: &BTreeSet<TopologyNodeKey>,
    options: &DotExportOptions,
) -> Vec<TopologyNodeKey> {
    let mut nodes = options
        .owned_bgp_nodes
        .iter()
        .filter_map(|node| visible_nodes.iter().find(|candidate| candidate.as_str() == node))
        .filter(|node| graphviz_node_has_bgp_state(node, graph))
        .cloned()
        .collect::<Vec<_>>();
    nodes.sort();
    nodes
}

/// Return non-BGP adjacency used for downstream recursive radial placement.
fn recursive_downstream_adjacency(
    edges: &[GraphvizEdge],
    visible_nodes: &BTreeSet<TopologyNodeKey>,
    options: &DotExportOptions,
) -> BTreeMap<TopologyNodeKey, Vec<TopologyNodeKey>> {
    let owned_bgp = options
        .owned_bgp_nodes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut adjacency = BTreeMap::<TopologyNodeKey, Vec<TopologyNodeKey>>::new();
    for edge in edges {
        if edge.link_kind == LinkKind::Bgp
            || !visible_nodes.contains(&edge.local_node)
            || !visible_nodes.contains(&edge.remote_node)
            || edge.local_node.as_str().starts_with("bgp:")
            || edge.remote_node.as_str().starts_with("bgp:")
            || owned_bgp.contains(edge.local_node.as_str())
            || owned_bgp.contains(edge.remote_node.as_str())
        {
            continue;
        }
        adjacency
            .entry(edge.local_node.clone())
            .or_default()
            .push(edge.remote_node.clone());
        adjacency
            .entry(edge.remote_node.clone())
            .or_default()
            .push(edge.local_node.clone());
    }
    for neighbors in adjacency.values_mut() {
        neighbors.sort();
        neighbors.dedup();
    }
    adjacency
}

/// Return a BFS tree from recursive radial adjacency.
fn recursive_radial_children(
    root: &TopologyNodeKey,
    adjacency: &BTreeMap<TopologyNodeKey, Vec<TopologyNodeKey>>,
) -> BTreeMap<TopologyNodeKey, Vec<TopologyNodeKey>> {
    let mut visited = BTreeSet::from([root.clone()]);
    let mut children = BTreeMap::<TopologyNodeKey, Vec<TopologyNodeKey>>::new();
    let mut queue = VecDeque::from([root.clone()]);
    while let Some(parent) = queue.pop_front() {
        let Some(neighbors) = adjacency.get(&parent) else {
            continue;
        };
        for neighbor in neighbors {
            if visited.insert(neighbor.clone()) {
                children.entry(parent.clone()).or_default().push(neighbor.clone());
                queue.push_back(neighbor.clone());
            }
        }
    }
    children
}

/// Place nodes over one or more arc rows and return the angle used for each node.
fn recursive_place_arc_rows(
    positions: &mut BTreeMap<TopologyNodeKey, (f64, f64)>,
    parent_position: (f64, f64),
    start: f64,
    end: f64,
    first_radius: f64,
    nodes: &[TopologyNodeKey],
) -> Vec<f64> {
    let mut angles_by_node = Vec::with_capacity(nodes.len());
    for (row, chunk) in nodes.chunks(GRAPHVIZ_RECURSIVE_RADIAL_MAX_ROW_CHILDREN).enumerate() {
        let radius = first_radius + usize_to_f64(row) * GRAPHVIZ_RECURSIVE_RADIAL_ROW_GAP;
        let angles = typed_radial_angles(start, end, chunk.len());
        for (node, angle) in chunk.iter().cloned().zip(angles.iter().copied()) {
            positions.insert(
                node,
                (
                    parent_position.0 + radius * angle.cos(),
                    parent_position.1 + radius * angle.sin(),
                ),
            );
            angles_by_node.push(angle);
        }
    }
    angles_by_node
}

/// Place disconnected nodes in two stable rows below the recursive topology.
fn recursive_place_disconnected_rows(positions: &mut BTreeMap<TopologyNodeKey, (f64, f64)>, nodes: &[TopologyNodeKey]) {
    if nodes.is_empty() {
        return;
    }
    let row_len = nodes.len().div_ceil(2).max(1);
    for (index, node) in nodes.iter().cloned().enumerate() {
        let row = index / row_len;
        let column = index % row_len;
        let current_row_len = (nodes.len() - row * row_len).min(row_len);
        let x = (usize_to_f64(column) - usize_to_f64(current_row_len.saturating_sub(1)) / 2.0)
            * GRAPHVIZ_RECURSIVE_RADIAL_DISCONNECTED_SPACING;
        let y = -GRAPHVIZ_RECURSIVE_RADIAL_DISCONNECTED_OFFSET - usize_to_f64(row) * GRAPHVIZ_RECURSIVE_RADIAL_ROW_GAP;
        positions.insert(node, (x, y));
    }
}
