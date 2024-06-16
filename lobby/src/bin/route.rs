use std::{
    collections::{BTreeMap, BinaryHeap},
    io::{self, Read},
    iter,
    ops::Deref,
    str::FromStr,
};

use clap::Parser;
use crossbeam_channel::Sender;
use itertools::Itertools;

type EdgeSet<'a> = BTreeMap<&'a str, (usize, Vec<&'a str>)>;
type Graph<'a> = BTreeMap<&'a str, EdgeSet<'a>>;
type WarpingGraph<'a> = BTreeMap<&'a str, (EdgeSet<'a>, EdgeSet<'a>)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Place {
    Vertex(usize),
    Warp(usize),
}

#[derive(Parser)]
#[command(version)]
struct Cli {
    collect: Option<usize>,
}

fn search_it<'a>(
    verts: &'a WarpingGraph,
    warps: &'a Graph,
    start: &str,
    end: &'a str,
) -> impl Iterator<Item = (usize, Vec<&'a str>)> {
    let verts_name_arr = verts.keys().map(|k| *k).collect::<Vec<_>>();
    let warps_name_arr = warps.keys().map(|k| *k).collect::<Vec<_>>();

    // collect graph into Vec, which can be treat as Map<usize, T>
    let verts_arr = verts_name_arr
        .iter()
        .map(|s| {
            let (to_v, to_w) = &verts[s];
            let to_v_arr = to_v
                .into_iter()
                .map(|(dst, &(d, ref act))| {
                    let dst_i = verts_name_arr
                        .iter()
                        .position(|s| s == dst)
                        .unwrap_or(verts_name_arr.len());
                    let act_arr = act
                        .into_iter()
                        .map(|s| warps_name_arr.iter().position(|ss| ss == s).unwrap())
                        .collect::<Vec<_>>()
                        .leak() as &[_];
                    (dst_i, d, act_arr)
                })
                .collect::<Vec<_>>()
                .leak() as &[_];
            let to_w_arr = to_w
                .into_iter()
                .map(|(dstw, &(d, ref act))| {
                    let dstw_i = warps_name_arr.iter().position(|s| s == dstw).unwrap();
                    let act_arr = act
                        .into_iter()
                        .map(|s| warps_name_arr.iter().position(|ss| ss == s).unwrap())
                        .collect::<Vec<_>>()
                        .leak() as &[_];
                    (dstw_i, d, act_arr)
                })
                .collect::<Vec<_>>()
                .leak() as &[_];
            (to_v_arr, to_w_arr)
        })
        .collect::<Vec<_>>()
        .leak() as &[_];
    let warps_arr = warps_name_arr
        .iter()
        .map(|s| {
            (&warps[s])
                .into_iter()
                .map(|(dst, &(d, ref act))| {
                    let dst_i = verts_name_arr
                        .iter()
                        .position(|s| s == dst)
                        .unwrap_or(verts_name_arr.len());
                    let act_arr = act
                        .into_iter()
                        .map(|s| warps_name_arr.iter().position(|ss| ss == s).unwrap())
                        .collect::<Vec<_>>()
                        .leak() as &[_];
                    (dst_i, d, act_arr)
                })
                .collect::<Vec<_>>()
                .leak() as &[_]
        })
        .collect::<Vec<_>>()
        .leak() as &[_];

    // map &str to usize
    let start_i = verts_name_arr.iter().position(|&s| s == start).unwrap();
    let end_i = verts_name_arr.len();

    // create stack buffer to build a path
    let mut rt_buf = Vec::<Place>::new();
    // create counter: Map<usize, Num> to determine whether a warp is activated
    let mut active_warps_buf = vec![0; warps.len()];

    // if a complete path is found, it will be sent through the channel
    let (sender, receiver) = crossbeam_channel::unbounded::<(usize, Vec<Place>)>();
    active_warps_buf.extend(iter::repeat(0).take(warps.len()));

    // single threaded algorithm, for locality
    fn single_threaded_search_vert(
        current_vert_i: usize,
        unexplored: usize,
        verts_arr: &[(&[(usize, usize, &[usize])], &[(usize, usize, &[usize])])],
        warps_arr: &[&[(usize, usize, &[usize])]],
        start_i: usize,
        end_i: usize,
        rt_buf: &mut Vec<Place>,
        total_dis: usize,
        active_warps_buf: &mut Vec<usize>,
        sender: &Sender<(usize, Vec<Place>)>,
    ) {
        //if exploration complete, go to the end
        if unexplored == 0 {
            let (to_v, to_w) = &verts_arr[current_vert_i];
            if let Some(&(dst, d, _)) = to_v.last() {
                if end_i == dst {
                    rt_buf.push(Place::Vertex(dst));
                    let len = total_dis + d;
                    sender.send((len, rt_buf.clone())).unwrap();
                    rt_buf.pop();
                }
            }
            if let Some(&(dstw, dw, _)) = to_w.first() {
                rt_buf.push(Place::Warp(dstw));
                for wi in 0..warps_arr.len() {
                    if active_warps_buf[wi] > 0 && wi != dstw {
                        let &(wdstv, wdv, _) = warps_arr[wi].last().unwrap();
                        if end_i == wdstv {
                            rt_buf.push(Place::Warp(wi));
                            rt_buf.push(Place::Vertex(wdstv));
                            let len = total_dis + dw + 69 + wdv;
                            sender.send((len, rt_buf.clone())).unwrap();
                            rt_buf.pop();
                            rt_buf.pop();
                        }
                    }
                }
                rt_buf.pop();
            }
        } else {
            //go to chapter
            let &(to_v, to_w) = &verts_arr[current_vert_i];
            for &(dst, d, act) in to_v {
                if !rt_buf.contains(&Place::Vertex(dst)) && end_i != dst {
                    rt_buf.push(Place::Vertex(dst));
                    for &a in act {
                        active_warps_buf[a] += 1;
                    }
                    single_threaded_search_vert(
                        dst,
                        unexplored - 1,
                        verts_arr,
                        warps_arr,
                        start_i,
                        end_i,
                        rt_buf,
                        total_dis + d,
                        active_warps_buf,
                        &sender,
                    );
                    for &a in act {
                        active_warps_buf[a] -= 1;
                    }
                    rt_buf.pop();
                }
            }
            // go to warp
            if let Some(&(dstw, dw, act)) = to_w.iter().max_by_key(|(.., act)| act.len()) {
                // activate warps
                for &a in act {
                    active_warps_buf[a] += 1;
                }
                rt_buf.push(Place::Warp(dstw));
                for wi in 0..warps_arr.len() {
                    // traverse active warps out, except for that has been just entered
                    if active_warps_buf[wi] > 0 && wi != dstw {
                        rt_buf.push(Place::Warp(wi));
                        let &w_to_v = &warps_arr[wi];
                        for &(wdstv, wdv, act) in w_to_v {
                            if !rt_buf.contains(&Place::Vertex(wdstv)) && end_i != wdstv {
                                rt_buf.push(Place::Vertex(wdstv));
                                for &a in act {
                                    active_warps_buf[a] += 1;
                                }
                                single_threaded_search_vert(
                                    wdstv,
                                    unexplored - 1,
                                    verts_arr,
                                    warps_arr,
                                    start_i,
                                    end_i,
                                    rt_buf,
                                    total_dis + dw + 69 + wdv,
                                    active_warps_buf,
                                    &sender,
                                );
                                for &a in act {
                                    active_warps_buf[a] -= 1;
                                }
                                rt_buf.pop();
                            }
                        }
                        rt_buf.pop();
                    }
                }
                rt_buf.pop();
                for &a in act {
                    active_warps_buf[a] -= 1;
                }
            }
            // go to start
            let &(s_to_v, _) = &verts_arr[start_i];
            rt_buf.push(Place::Vertex(start_i));
            for &(dst, d, act) in s_to_v {
                if !rt_buf.contains(&Place::Vertex(dst))
                    && to_v.binary_search_by_key(&dst, |&(d, ..)| d).is_err()
                {
                    rt_buf.push(Place::Vertex(dst));
                    for &a in act {
                        active_warps_buf[a] += 1;
                    }
                    single_threaded_search_vert(
                        dst,
                        unexplored - 1,
                        verts_arr,
                        warps_arr,
                        start_i,
                        end_i,
                        rt_buf,
                        total_dis + d + 69,
                        active_warps_buf,
                        &sender,
                    );
                    for &a in act {
                        active_warps_buf[a] -= 1;
                    }
                    rt_buf.pop();
                }
            }
            rt_buf.pop();
        }
    }

    // number for the determination of forking
    let workers_num = rayon::current_num_threads();
    const WORKER_FACTOR: usize = 32;

    fn search_vert(
        current_vert_i: usize,
        unexplored: usize,
        verts_arr: &'static [(&[(usize, usize, &[usize])], &[(usize, usize, &[usize])])],
        warps_arr: &'static [&[(usize, usize, &[usize])]],
        start_i: usize,
        end_i: usize,
        rt_buf: &mut Vec<Place>,
        total_dis: usize,
        active_warps_buf: &mut Vec<usize>,
        sender: Sender<(usize, Vec<Place>)>,
        estimated_extent: usize,
        workers_num: usize,
    ) {
        //if exploration complete, go to the end
        if unexplored == 0 {
            let (to_v, to_w) = &verts_arr[current_vert_i];
            if let Some(&(dst, d, _)) = to_v.last() {
                if end_i == dst {
                    rt_buf.push(Place::Vertex(dst));
                    let len = total_dis + d;
                    sender.send((len, rt_buf.clone())).unwrap();
                    rt_buf.pop();
                }
            }
            if let Some(&(dstw, dw, _)) = to_w.first() {
                for wi in 0..warps_arr.len() {
                    if active_warps_buf[wi] > 0 && wi != dstw {
                        let &(wdstv, wdv, _) = warps_arr[wi].last().unwrap();
                        if end_i == wdstv {
                            rt_buf.push(Place::Warp(wi));
                            rt_buf.push(Place::Vertex(wdstv));
                            let len = total_dis + dw + 69 + wdv;
                            sender.send((len, rt_buf.clone())).unwrap();
                            rt_buf.pop();
                            rt_buf.pop();
                        }
                    }
                }
            }
        } else {
            //go to chapter
            let &(to_v, to_w) = &verts_arr[current_vert_i];
            for &(dst, d, act) in to_v {
                if !rt_buf.contains(&Place::Vertex(dst)) && end_i != dst {
                    rt_buf.push(Place::Vertex(dst));
                    for &a in act {
                        active_warps_buf[a] += 1;
                    }
                    let estimated_extent = to_v.len() * estimated_extent;
                    // determine if tree extent is large enough to stop forking
                    if estimated_extent < workers_num * WORKER_FACTOR {
                        let mut rt_buf = rt_buf.clone();
                        let mut active_warps_buf = active_warps_buf.clone();
                        let collector = sender.clone();
                        rayon::spawn(move || {
                            search_vert(
                                dst,
                                unexplored - 1,
                                verts_arr,
                                warps_arr,
                                start_i,
                                end_i,
                                &mut rt_buf,
                                total_dis + d,
                                &mut active_warps_buf,
                                collector,
                                estimated_extent,
                                workers_num,
                            );
                        });
                    } else {
                        single_threaded_search_vert(
                            dst,
                            unexplored - 1,
                            verts_arr,
                            warps_arr,
                            start_i,
                            end_i,
                            rt_buf,
                            total_dis + d,
                            active_warps_buf,
                            &sender,
                        );
                    }
                    for &a in act {
                        active_warps_buf[a] -= 1;
                    }
                    rt_buf.pop();
                }
            }
            // go to warp
            if let Some(&(dstw, dw, act)) = to_w.iter().max_by_key(|(.., act)| act.len()) {
                // activate warps
                for &a in act {
                    active_warps_buf[a] += 1;
                }
                rt_buf.push(Place::Warp(dstw));
                for wi in 0..warps_arr.len() {
                    // traverse active warps out, except for that has been just entered
                    if active_warps_buf[wi] > 0 && wi != dstw {
                        let &w_to_v = &warps_arr[wi];
                        rt_buf.push(Place::Warp(wi));
                        for &(wdstv, wdv, act) in w_to_v {
                            if !rt_buf.contains(&Place::Vertex(wdstv)) && end_i != wdstv {
                                rt_buf.push(Place::Vertex(wdstv));
                                for &a in act {
                                    active_warps_buf[a] += 1;
                                }
                                let estimated_extent = w_to_v.len() * estimated_extent;
                                if estimated_extent < workers_num * WORKER_FACTOR {
                                    let mut rt_buf = rt_buf.clone();
                                    let mut active_warps_buf = active_warps_buf.clone();
                                    let collector = sender.clone();
                                    rayon::spawn(move || {
                                        search_vert(
                                            wdstv,
                                            unexplored - 1,
                                            verts_arr,
                                            warps_arr,
                                            start_i,
                                            end_i,
                                            &mut rt_buf,
                                            total_dis + dw + 69 + wdv,
                                            &mut active_warps_buf,
                                            collector,
                                            estimated_extent,
                                            workers_num,
                                        );
                                    })
                                } else {
                                    single_threaded_search_vert(
                                        wdstv,
                                        unexplored - 1,
                                        verts_arr,
                                        warps_arr,
                                        start_i,
                                        end_i,
                                        rt_buf,
                                        total_dis + dw + 69 + wdv,
                                        active_warps_buf,
                                        &sender,
                                    );
                                }
                                for &a in act {
                                    active_warps_buf[a] -= 1;
                                }
                                rt_buf.pop();
                            }
                        }
                        rt_buf.pop();
                    }
                }
                rt_buf.pop();
                for &a in act {
                    active_warps_buf[a] -= 1;
                }
            }
            // go to start
            let &(s_to_v, _) = &verts_arr[start_i];
            rt_buf.push(Place::Vertex(start_i));
            for &(dst, d, act) in s_to_v {
                if !rt_buf.contains(&Place::Vertex(dst))
                    && to_v.binary_search_by_key(&dst, |&(d, ..)| d).is_err()
                {
                    rt_buf.push(Place::Vertex(dst));
                    for &a in act {
                        active_warps_buf[a] += 1;
                    }
                    let estimated_extent = s_to_v.len() * estimated_extent;
                    if estimated_extent < workers_num * WORKER_FACTOR {
                        let mut rt_buf = rt_buf.clone();
                        let mut active_warps_buf = active_warps_buf.clone();
                        let collector = sender.clone();
                        rayon::spawn(move || {
                            search_vert(
                                dst,
                                unexplored - 1,
                                verts_arr,
                                warps_arr,
                                start_i,
                                end_i,
                                &mut rt_buf,
                                total_dis + d + 69,
                                &mut active_warps_buf,
                                collector,
                                estimated_extent,
                                workers_num,
                            );
                        });
                    } else {
                        single_threaded_search_vert(
                            dst,
                            unexplored - 1,
                            verts_arr,
                            warps_arr,
                            start_i,
                            end_i,
                            rt_buf,
                            total_dis + d + 69,
                            active_warps_buf,
                            &sender,
                        );
                    }
                    for &a in act {
                        active_warps_buf[a] -= 1;
                    }
                    rt_buf.pop();
                }
            }
            rt_buf.pop();
        }
    }

    rayon::spawn(move || {
        rt_buf.push(Place::Vertex(start_i));
        search_vert(
            start_i,
            verts_arr.len() - 1,
            verts_arr,
            warps_arr,
            start_i,
            end_i,
            &mut rt_buf,
            0,
            &mut active_warps_buf,
            sender,
            1,
            workers_num,
        );
    });

    receiver.into_iter().map(move |(total_dis, p)| {
        let p = p
            .into_iter()
            .map(|i| match i {
                Place::Vertex(v) => {
                    if v == end_i {
                        end
                    } else {
                        verts_name_arr[v]
                    }
                }
                Place::Warp(w) => warps_name_arr[w],
            })
            .collect();
        (total_dis, p)
    })
}

fn main() {
    let cli = Cli::parse();
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();
    let data = String::from_utf8(buf).unwrap();
    let mut graph: Graph = serde_json::from_str(&data).unwrap();
    let warps = graph.split_off("A");
    let verts: WarpingGraph = graph
        .into_iter()
        .map(|(k, mut v)| {
            let vm1 = v.split_off("A");
            (k, (v, vm1))
        })
        .collect();
    let (min, max) = verts
        .iter()
        .flat_map(|(&k, (vt, _))| iter::once(k).chain(vt.keys().map(Deref::deref)))
        .chain(warps.iter().flat_map(|(_, v)| v.keys().map(Deref::deref)))
        .filter_map(|s| usize::from_str(s).ok().map(|index| (index, s)))
        .minmax()
        .into_option()
        .map(|((_, min), (_, max))| (min, max))
        .unwrap();

    let collect_count = cli.collect.unwrap_or(1);
    let mut res_buf: BinaryHeap<(usize, Vec<&str>)> = BinaryHeap::new();
    let mut rt_count = 0;

    for e in search_it(&verts, &warps, min, max) {
        rt_count += 1;
        res_buf.push(e);
        while res_buf.len() > collect_count {
            res_buf.pop();
        }
    }

    println!("Found {rt_count} routes.");
    for (i, (dis, rt)) in res_buf.into_sorted_vec().into_iter().enumerate() {
        println!("({}) [{}] {}", i + 1, dis, rt.join("-"));
    }
}
