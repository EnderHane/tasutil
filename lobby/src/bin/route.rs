use std::{
    borrow::Borrow,
    collections::{BTreeMap, BinaryHeap},
    io,
    marker::PhantomData,
    sync::Arc,
};

use clap::Parser;
use itertools::Itertools;

type EdgeSet<'a> = BTreeMap<&'a str, (usize, Vec<&'a str>)>;
type Graph<'a> = BTreeMap<&'a str, EdgeSet<'a>>;
type BiGraph<'a> = BTreeMap<&'a str, (EdgeSet<'a>, EdgeSet<'a>)>;

type Edge = (usize, usize, Box<[usize]>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Place {
    Vertex(usize),
    Warp(usize),
}

type SearchResultEntry = (usize, Vec<Place>);

#[derive(Clone, Default)]
struct SearchState {
    route: Vec<Place>,
    active_warps: Vec<usize>,
}

impl SearchState {
    fn new(warp_num: usize) -> Self {
        Self {
            route: Vec::new(),
            active_warps: vec![0; warp_num],
        }
    }
}

#[derive(Debug)]
struct Searcher {
    vertexes: Box<[(Box<[Edge]>, Box<[Edge]>)]>,
    warps: Box<[Box<[Edge]>]>,
}

trait Report {
    type Res;
    fn send(&mut self, res: Self::Res);
}

impl Searcher {
    const START_I: usize = 0;
    const END_I: usize = isize::MAX as usize + 1;

    fn search_place(
        &self,
        current_vertex: usize,
        unexpl: usize,
        excur: usize,
        st: &mut SearchState,
        collector: &mut impl Report<Res = SearchResultEntry>,
    ) {
        let (to_v, to_w) = &self.vertexes[current_vertex];
        if unexpl == 0 {
            // go to end
            let mut any_ended = false;
            if let Some(&(e @ Self::END_I, d, _)) = to_v.last() {
                let mut r = st.route.clone();
                r.push(Place::Vertex(e));
                let excur = excur + d;
                collector.send((excur, r));
                any_ended = true;
            }
            if let Some(&(dst, d, ref act)) = to_w.first().filter(|_| !any_ended) {
                st.route.push(Place::Warp(dst));
                for &aw in act.iter() {
                    st.active_warps[aw] += 1;
                }
                let excur = excur + d + 69;
                for w in 0..self.warps.len() {
                    if st.active_warps[w] > 0 && w != dst {
                        let w_to_v = &self.warps[w];
                        st.route.push(Place::Warp(w));
                        if let Some(&(e @ Self::END_I, d, _)) = w_to_v.last() {
                            let mut r = st.route.clone();
                            r.push(Place::Vertex(e));
                            let excur = excur + d;
                            collector.send((excur, r));
                            any_ended = true;
                        }
                        st.route.pop();
                    }
                }
                st.route.pop();
                for &aw in act.iter() {
                    st.active_warps[aw] -= 1;
                }
            }
            if !any_ended {
                let (s_to_v, s_to_w) = &self.vertexes[Self::START_I];
                st.route.push(Place::Vertex(Self::START_I));
                if let Some(&(e @ Self::END_I, d, _)) = s_to_v.last() {
                    let mut r = st.route.clone();
                    r.push(Place::Vertex(e));
                    let excur = excur + d;
                    collector.send((excur, r));
                    any_ended = true;
                }
                if let Some(&(dst, d, ref act)) = s_to_w.first().filter(|_| !any_ended) {
                    st.route.push(Place::Warp(dst));
                    for &aw in act.iter() {
                        st.active_warps[aw] += 1;
                    }
                    let excur = excur + d + 69;
                    for w in 0..self.warps.len() {
                        if st.active_warps[w] > 0 && w != dst {
                            let w_to_v = &self.warps[w];
                            st.route.push(Place::Warp(w));
                            if let Some(&(e @ Self::END_I, d, _)) = w_to_v.last() {
                                let mut r = st.route.clone();
                                r.push(Place::Vertex(e));
                                let excur = excur + d;
                                collector.send((excur, r));
                                // any_ended = true;
                            }
                            st.route.pop();
                        }
                    }
                    st.route.pop();
                    for &aw in act.iter() {
                        st.active_warps[aw] -= 1;
                    }
                }
                st.route.pop();
            }
        } else {
            let mut any_searched = false;
            // go to next
            for &(dst, d, ref act) in to_v.iter() {
                if !st.route.contains(&Place::Vertex(dst)) && dst != Self::END_I {
                    st.route.push(Place::Vertex(dst));
                    for &aw in act.iter() {
                        st.active_warps[aw] += 1;
                    }
                    let unexpl = unexpl - 1;
                    let excur = excur + d;
                    any_searched = true;
                    self.search_place(dst, unexpl, excur, st, collector);
                    st.route.pop();
                    for &aw in act.iter() {
                        st.active_warps[aw] -= 1;
                    }
                }
            }
            // go to next warp
            if let Some(&(dst, d, ref act)) = to_w.first().filter(|_| !any_searched) {
                st.route.push(Place::Warp(dst));
                for &aw in act.iter() {
                    st.active_warps[aw] += 1;
                }
                let excur = excur + d + 69;
                for w in 0..self.warps.len() {
                    if st.active_warps[w] > 0 && w != dst {
                        let w_to_v = &self.warps[w];
                        st.route.push(Place::Warp(w));
                        for &(dst, d, ref act) in w_to_v.iter() {
                            if dst != Self::END_I
                                && to_v.binary_search_by_key(&dst, |&(dst1, ..)| dst1).is_err()
                                && !st.route.contains(&Place::Vertex(dst))
                            {
                                st.route.push(Place::Vertex(dst));
                                for &aw in act.iter() {
                                    st.active_warps[aw] += 1;
                                }
                                let unexpl = unexpl - 1;
                                let excur = excur + d;
                                any_searched = true;
                                self.search_place(dst, unexpl, excur, st, collector);
                                st.route.pop();
                                for &aw in act.iter() {
                                    st.active_warps[aw] -= 1;
                                }
                            }
                        }
                        st.route.pop();
                    }
                }
                st.route.pop();
                for &aw in act.iter() {
                    st.active_warps[aw] -= 1;
                }
            }
            // go to start
            if !any_searched {
                let (s_to_v, s_to_w) = &self.vertexes[Self::START_I];
                st.route.push(Place::Vertex(Self::START_I));
                let excur = excur + 69;
                for &(dst, d, ref act) in s_to_v.iter() {
                    if dst != Self::END_I
                        && to_v.binary_search_by_key(&dst, |&(dst1, ..)| dst1).is_err()
                        && !st.route.contains(&Place::Vertex(dst))
                    {
                        st.route.push(Place::Vertex(dst));
                        for &aw in act.iter() {
                            st.active_warps[aw] += 1;
                        }
                        let unexpl = unexpl - 1;
                        let excur = excur + d;
                        any_searched = true;
                        self.search_place(dst, unexpl, excur, st, collector);
                        st.route.pop();
                        for &aw in act.iter() {
                            st.active_warps[aw] -= 1;
                        }
                    }
                }
                if let Some(&(dst, d, ref act)) = s_to_w.first().filter(|_| !any_searched) {
                    st.route.push(Place::Warp(dst));
                    for &aw in act.iter() {
                        st.active_warps[aw] += 1;
                    }
                    let excur = excur + d + 69;
                    for w in 0..self.warps.len() {
                        if st.active_warps[w] > 0 && w != dst {
                            let w_to_v = &self.warps[w];
                            st.route.push(Place::Warp(w));
                            for &(dst, d, ref act) in w_to_v.iter() {
                                if dst != Self::END_I
                                    && to_v.binary_search_by_key(&dst, |&(dst1, ..)| dst1).is_err()
                                    && !st.route.contains(&Place::Vertex(dst))
                                {
                                    st.route.push(Place::Vertex(dst));
                                    for &aw in act.iter() {
                                        st.active_warps[aw] += 1;
                                    }
                                    let unexpl = unexpl - 1;
                                    let excur = excur + d;
                                    self.search_place(dst, unexpl, excur, st, collector);
                                    st.route.pop();
                                    for &aw in act.iter() {
                                        st.active_warps[aw] -= 1;
                                    }
                                }
                            }
                            st.route.pop();
                        }
                    }
                    st.route.pop();
                    for &aw in act.iter() {
                        st.active_warps[aw] -= 1;
                    }
                }
                st.route.pop();
            }
        }
    }

    fn search(&self, collector: &mut impl Report<Res = SearchResultEntry>) {
        let mut st = SearchState::new(self.warps.len());
        let st = &mut st;
        let excur = 0;
        let unexpl = self.vertexes.len() - 1;
        // go to start
        let mut any_searched = false;
        let (s_to_v, s_to_w) = &self.vertexes[Self::START_I];
        st.route.push(Place::Vertex(Self::START_I));
        for &(dst, d, ref act) in s_to_v.iter() {
            if dst != Self::END_I && !st.route.contains(&Place::Vertex(dst)) {
                st.route.push(Place::Vertex(dst));
                for &aw in act.iter() {
                    st.active_warps[aw] += 1;
                }
                let unexpl = unexpl - 1;
                let excur = excur + d;
                any_searched = true;
                self.search_place(dst, unexpl, excur, st, collector);
                st.route.pop();
                for &aw in act.iter() {
                    st.active_warps[aw] -= 1;
                }
            }
        }
        if let Some(&(dst, d, ref act)) = s_to_w.first().filter(|_| !any_searched) {
            st.route.push(Place::Warp(dst));
            for &aw in act.iter() {
                st.active_warps[aw] += 1;
            }
            let excur = excur + d + 69;
            for w in 0..self.warps.len() {
                if st.active_warps[w] > 0 && w != dst {
                    let w_to_v = &self.warps[w];
                    for &(dst, d, ref act) in w_to_v.iter() {
                        if dst != Self::END_I && !st.route.contains(&Place::Vertex(dst)) {
                            st.route.push(Place::Vertex(dst));
                            for &aw in act.iter() {
                                st.active_warps[aw] += 1;
                            }
                            let unexpl = unexpl - 1;
                            let excur = excur + d;
                            // any_searched = true;
                            self.search_place(dst, unexpl, excur, st, collector);
                            st.route.pop();
                            for &aw in act.iter() {
                                st.active_warps[aw] -= 1;
                            }
                        }
                    }
                }
            }
            st.route.pop();
            for &aw in act.iter() {
                st.active_warps[aw] -= 1;
            }
        }
        st.route.pop();
    }
}

#[derive(Debug)]
struct SearcherBuilder<'a, 's: 'a> {
    splited: &'a VertexWarpSplit<'s>,
    thread_num: Option<usize>,
}

impl<'a, 's: 'a> SearcherBuilder<'a, 's> {
    fn new(splited: &'a VertexWarpSplit<'s>) -> Self {
        Self {
            splited,
            thread_num: Default::default(),
        }
    }

    fn thread_num(self, n: usize) -> Self {
        Self {
            thread_num: Some(n),
            ..self
        }
    }

    fn build(self) -> (PlaceName<'s>, Searcher) {
        let place_name = PlaceName::new(self.splited);

        let vertexes = place_name
            .inters
            .iter()
            .map(|src| {
                let to_v = self.splited.v()[src]
                    .0
                    .iter()
                    .map(|(dst, (d, act))| {
                        let dst = place_name
                            .inters
                            .binary_search(dst)
                            .unwrap_or(Searcher::END_I);
                        let act = act
                            .iter()
                            .map(|w| place_name.warps.binary_search(w).unwrap())
                            .collect::<Box<_>>();
                        (dst, *d, act)
                    })
                    .collect();

                let to_w = self.splited.v()[src]
                    .1
                    .iter()
                    .map(|(dst, (d, act))| {
                        let dst = place_name.warps.binary_search(dst).unwrap();
                        let act = act
                            .iter()
                            .map(|w| place_name.warps.binary_search(w).unwrap())
                            .collect::<Box<_>>();
                        (dst, *d, act)
                    })
                    .collect();
                (to_v, to_w)
            })
            .collect();

        let warps = place_name
            .warps
            .iter()
            .map(|src| {
                self.splited
                    .w()
                    .get(src)
                    .iter()
                    .flat_map(|i| i.into_iter())
                    .map(|(dst, (d, act))| {
                        let dst = place_name
                            .inters
                            .binary_search(dst)
                            .unwrap_or(Searcher::END_I);
                        let act = act
                            .iter()
                            .map(|w| place_name.warps.binary_search(w).unwrap())
                            .collect();
                        (dst, *d, act)
                    })
                    .collect()
            })
            .collect();
        let s = Searcher {
            vertexes,
            warps,
            //_phantom: PhantomData,
        };

        (place_name, s)
    }
}

#[derive(Debug)]
struct VertexWarpSplit<'a> {
    vertexes: BiGraph<'a>,
    warps: Graph<'a>,
}

impl<'a> VertexWarpSplit<'a> {
    fn from_graph(mut graph: Graph<'a>) -> Self {
        let warps = graph.split_off("A");
        let (warp_to_vertex, _) = warps
            .into_iter()
            .map(|(wsrc, mut dsts)| {
                let (wdsts, vdsts) = (dsts.split_off("A"), dsts);
                ((wsrc, vdsts), (wsrc, wdsts))
            })
            .unzip::<_, _, Graph, Graph>();
        let vertexes = graph
            .into_iter()
            .map(|(src, mut dsts)| {
                let (wdsts, vdsts) = (dsts.split_off("A"), dsts);
                (src, (vdsts, wdsts))
            })
            .collect();
        Self {
            vertexes,
            warps: warp_to_vertex,
        }
    }

    fn v(&self) -> &BiGraph<'a> {
        &self.vertexes
    }

    fn w(&self) -> &Graph<'a> {
        &self.warps
    }
}

#[derive(Debug)]
struct PlaceName<'a> {
    inters: Vec<&'a str>,
    end: &'a str,
    warps: Vec<&'a str>,
}

impl<'a> PlaceName<'a> {
    fn new(vertex_and_warp: &VertexWarpSplit<'a>) -> Self {
        let inters = vertex_and_warp
            .v()
            .keys()
            .map(|&src| src)
            .collect::<Vec<_>>();
        let end = vertex_and_warp
            .v()
            .values()
            .map(|(v, _)| v)
            .chain(vertex_and_warp.w().values())
            .flat_map(IntoIterator::into_iter)
            .map(|(&vdst, _)| vdst)
            .filter(|s| !inters.contains(s))
            .unique()
            .exactly_one()
            .expect("Cannot determine destination. The lobby might be incomplete.");
        let warps = vertex_and_warp
            .v()
            .values()
            .flat_map(|(_, w)| w.into_iter())
            .map(|(dst, _)| dst)
            .chain(vertex_and_warp.w().keys())
            .map(|&wsrc| wsrc)
            .unique()
            .sorted()
            .collect::<Vec<_>>();
        Self { end, inters, warps }
    }
}

impl PlaceName<'_> {
    // fn start(&self) -> &str {
    //     self.inters[Searcher::START_I]
    // }

    fn name(&self, place: Place) -> &str {
        match place {
            Place::Vertex(v) => self.inters.get(v).unwrap_or(&self.end),
            Place::Warp(w) => self.warps[w],
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let max_collect = cli
        .collect
        .inspect(|&v| {
            if v > 1000 {
                eprintln!("Warning: Collecting amount will be clamped to 1000")
            }
        })
        .unwrap_or(3)
        .clamp(1, 1000);

    let mut buf = String::new();
    let mut graph: Option<Graph> = None;
    for line_input in io::stdin().lines() {
        buf.push_str(&line_input.expect("Receive error reading input"));
        graph = serde_json::from_str(&buf).ok();
    }
    let graph = graph.expect("Fail to deserialize");

    let splited: VertexWarpSplit = VertexWarpSplit::from_graph(graph);
    let (pn, s) = SearcherBuilder::new(&splited).build();
    struct Listener {
        count: usize,
        max_collect: usize,
        res_buf: BinaryHeap<SearchResultEntry>,
    }
    impl Report for Listener {
        type Res = SearchResultEntry;

        fn send(&mut self, res: Self::Res) {
            self.res_buf.push(res);
            self.count += 1;
            while self.res_buf.len() > self.max_collect {
                self.res_buf.pop();
            }
        }
    }
    let mut listener = Listener {
        count: 0,
        max_collect,
        res_buf: BinaryHeap::new(),
    };
    s.search(&mut listener);
    let res = listener.res_buf.into_sorted_vec();
    println!("Found {} routes", listener.count);
    for (i, (l, v)) in res.into_iter().enumerate() {
        let pth = v.iter().map(|&pl| pn.name(pl)).join("-");
        println!("({})\t[{l}]\t{pth}", i + 1);
    }
}

#[derive(Parser)]
#[command(version)]
struct Cli {
    collect: Option<usize>,
    #[arg(id = "threads", short, long)]
    threads: Option<Option<usize>>,
}
