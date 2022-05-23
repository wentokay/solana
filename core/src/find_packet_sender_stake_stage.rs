use {
    crossbeam_channel::{Receiver, RecvTimeoutError, Sender},
    rayon::{prelude::*, ThreadPool},
    solana_gossip::cluster_info::ClusterInfo,
    solana_measure::measure::Measure,
    solana_perf::packet::PacketBatch,
    solana_rayon_threadlimit::get_thread_count,
    solana_runtime::bank_forks::BankForks,
    solana_sdk::timing::timestamp,
    solana_streamer::streamer::{self, StreamerError},
    std::{
        cell::RefCell,
        collections::HashMap,
        net::IpAddr,
        sync::{Arc, RwLock},
        thread::{self, Builder, JoinHandle},
        time::{Duration, Instant},
    },
};

const IP_TO_STAKE_REFRESH_DURATION: Duration = Duration::from_secs(5);

thread_local!(static PAR_THREAD_POOL: RefCell<ThreadPool> = RefCell::new(rayon::ThreadPoolBuilder::new()
                    .num_threads(get_thread_count())
                    .thread_name(|ix| format!("transaction_sender_stake_stage_{}", ix))
                    .build()
                    .unwrap()));

// Try to target 50ms, rough timings from mainnet machines
//
// 50ms/(1us/packet) = 50k packets
const MAX_FINDPACKETSENDERSTAKE_BATCH: usize = 50_000;

pub type FindPacketSenderStakeSender = Sender<Vec<PacketBatch>>;
pub type FindPacketSenderStakeReceiver = Receiver<Vec<PacketBatch>>;

#[derive(Debug, Default)]
struct FindPacketSenderStakeStats {
    last_print: u64,
    refresh_ip_to_stake_time: u64,
    apply_sender_stakes_time: u64,
    send_batches_time: u64,
    receive_batches_time: u64,
    total_batches: u64,
    total_packets: u64,
    total_discard_random: usize,
    total_discard_random_time_us: usize,
}

impl FindPacketSenderStakeStats {
    fn report(&mut self, name: &'static str) {
        let now = timestamp();
        let elapsed_ms = now - self.last_print;
        if elapsed_ms > 2000 {
            datapoint_info!(
                name,
                (
                    "refresh_ip_to_stake_time_us",
                    self.refresh_ip_to_stake_time as i64,
                    i64
                ),
                (
                    "apply_sender_stakes_time_us",
                    self.apply_sender_stakes_time as i64,
                    i64
                ),
                ("send_batches_time_us", self.send_batches_time as i64, i64),
                (
                    "receive_batches_time_ns",
                    self.receive_batches_time as i64,
                    i64
                ),
                ("total_batches", self.total_batches as i64, i64),
                ("total_packets", self.total_packets as i64, i64),
                ("total_discard_random", self.total_discard_random, i64),
                (
                    "total_discard_random_time_us",
                    self.total_discard_random_time_us,
                    i64
                ),
            );
            *self = FindPacketSenderStakeStats::default();
            self.last_print = now;
        }
    }
}

pub struct FindPacketSenderStakeStage {
    thread_hdl: JoinHandle<()>,
}

impl FindPacketSenderStakeStage {
    pub fn new(
        packet_receiver: streamer::PacketBatchReceiver,
        sender: FindPacketSenderStakeSender,
        bank_forks: Arc<RwLock<BankForks>>,
        cluster_info: Arc<ClusterInfo>,
        name: &'static str,
    ) -> Self {
        let mut stats = FindPacketSenderStakeStats::default();
        let thread_hdl = Builder::new()
            .name("find-packet-sender-stake".to_string())
<<<<<<< HEAD
            .spawn(move || {
                let mut last_stakes = Instant::now();
                let mut ip_to_stake: HashMap<IpAddr, u64> = HashMap::new();
                loop {
                    let mut refresh_ip_to_stake_time = Measure::start("refresh_ip_to_stake_time");
                    Self::try_refresh_ip_to_stake(
                        &mut last_stakes,
                        &mut ip_to_stake,
                        bank_forks.clone(),
                        cluster_info.clone(),
                    );
                    refresh_ip_to_stake_time.stop();
                    stats.refresh_ip_to_stake_time = stats
                        .refresh_ip_to_stake_time
                        .saturating_add(refresh_ip_to_stake_time.as_us());

                    match streamer::recv_packet_batches(&packet_receiver) {
                        Ok((mut batches, num_packets, recv_duration)) => {
                            let num_batches = batches.len();
                            let mut apply_sender_stakes_time =
                                Measure::start("apply_sender_stakes_time");
=======
            .spawn(move || loop {
                match streamer::recv_packet_batches(&packet_receiver) {
                    Ok((mut batches, num_packets, recv_duration)) => {
                        let num_batches = batches.len();

                        let mut discard_random_time =
                            Measure::start("findpacketsenderstake_discard_random_time");
                        let non_discarded_packets = solana_perf::discard::discard_batches_randomly(
                            &mut batches,
                            MAX_FINDPACKETSENDERSTAKE_BATCH,
                            num_packets,
                        );
                        let num_discarded_randomly =
                            num_packets.saturating_sub(non_discarded_packets);
                        discard_random_time.stop();

                        let mut apply_sender_stakes_time =
                            Measure::start("apply_sender_stakes_time");
                        let mut apply_stake = || {
                            let ip_to_stake = staked_nodes.read().unwrap();
>>>>>>> 6429aff13 (findpacketsenderstake: add discard after receive (#25458))
                            Self::apply_sender_stakes(&mut batches, &ip_to_stake);
                            apply_sender_stakes_time.stop();

                            let mut send_batches_time = Measure::start("send_batches_time");
                            if let Err(e) = sender.send(batches) {
                                info!("Sender error: {:?}", e);
                            }
                            send_batches_time.stop();

<<<<<<< HEAD
                            stats.apply_sender_stakes_time = stats
                                .apply_sender_stakes_time
                                .saturating_add(apply_sender_stakes_time.as_us());
                            stats.send_batches_time = stats
                                .send_batches_time
                                .saturating_add(send_batches_time.as_us());
                            stats.receive_batches_time = stats
                                .receive_batches_time
                                .saturating_add(recv_duration.as_nanos() as u64);
                            stats.total_batches =
                                stats.total_batches.saturating_add(num_batches as u64);
                            stats.total_packets =
                                stats.total_packets.saturating_add(num_packets as u64);
                        }
                        Err(e) => match e {
                            StreamerError::RecvTimeout(RecvTimeoutError::Disconnected) => break,
                            StreamerError::RecvTimeout(RecvTimeoutError::Timeout) => (),
                            _ => error!("error: {:?}", e),
                        },
=======
                        stats.apply_sender_stakes_time = stats
                            .apply_sender_stakes_time
                            .saturating_add(apply_sender_stakes_time.as_us());
                        stats.send_batches_time = stats
                            .send_batches_time
                            .saturating_add(send_batches_time.as_us());
                        stats.receive_batches_time = stats
                            .receive_batches_time
                            .saturating_add(recv_duration.as_nanos() as u64);
                        stats.total_batches =
                            stats.total_batches.saturating_add(num_batches as u64);
                        stats.total_packets =
                            stats.total_packets.saturating_add(num_packets as u64);
                        stats.total_discard_random_time_us += discard_random_time.as_us() as usize;
                        stats.total_discard_random += num_discarded_randomly;
>>>>>>> 6429aff13 (findpacketsenderstake: add discard after receive (#25458))
                    }

                    stats.report(name);
                }
            })
            .unwrap();
        Self { thread_hdl }
    }

    fn try_refresh_ip_to_stake(
        last_stakes: &mut Instant,
        ip_to_stake: &mut HashMap<IpAddr, u64>,
        bank_forks: Arc<RwLock<BankForks>>,
        cluster_info: Arc<ClusterInfo>,
    ) {
        if last_stakes.elapsed() > IP_TO_STAKE_REFRESH_DURATION {
            let root_bank = bank_forks.read().unwrap().root_bank();
            let staked_nodes = root_bank.staked_nodes();
            *ip_to_stake = cluster_info
                .tvu_peers()
                .into_iter()
                .filter_map(|node| {
                    let stake = staked_nodes.get(&node.id)?;
                    Some((node.tvu.ip(), *stake))
                })
                .collect();
            *last_stakes = Instant::now();
        }
    }

    fn apply_sender_stakes(batches: &mut [PacketBatch], ip_to_stake: &HashMap<IpAddr, u64>) {
        PAR_THREAD_POOL.with(|thread_pool| {
            thread_pool.borrow().install(|| {
                batches
                    .into_par_iter()
                    .flat_map(|batch| batch.packets.par_iter_mut())
                    .for_each(|packet| {
                        packet.meta.sender_stake =
                            *ip_to_stake.get(&packet.meta.addr().ip()).unwrap_or(&0);
                    });
            })
        });
    }

    pub fn join(self) -> thread::Result<()> {
        self.thread_hdl.join()
    }
}
