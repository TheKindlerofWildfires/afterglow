use std::time::{Duration, SystemTime};

use crate::{
    core::{channel::MAX_PACKET_SIZE, SYN_INTERVAL},
    utils::{self, SequenceNumber},
};

#[derive(Debug)]
pub struct CongestionController {
    pkt_send_period: Duration,
    congestion_window: usize, //packets
    max_congestion_window: usize,
    bandwidth: usize, //packets per second
    mss: usize,
    cur_send_seq_no: SequenceNumber,
    recv_rate: usize, //packets per second
    rtt: Duration,
    rtt_var: Duration,
    ack_interval: usize,
    rc_interval: Duration,
    last_rc_time: SystemTime,
    slow_start: bool,
    last_ack_seq_no: SequenceNumber,
    loss: bool,
    last_dec_seq_no: SequenceNumber,
    last_dec_period: Duration,
    loss_count: usize,
    dec_random: usize,
    avg_loss: usize,
    dec_count: usize,
    pkt_count: usize,
}

impl CongestionController {
    pub fn new() -> Self {
        let pkt_send_period = Duration::from_micros(1);
        let congestion_window = 16;
        let max_congestion_window = 16;
        let bandwidth = 1;
        let mss = MAX_PACKET_SIZE as usize;
        let cur_send_seq_no = SequenceNumber::MAX_SEQ_NO;
        let recv_rate = 16;
        let rtt = Duration::from_micros(10);
        let rtt_var = Duration::from_micros(1);
        let ack_interval = 0;
        let rc_interval = Duration::from_micros(10_000);
        let last_rc_time = SystemTime::now();
        let slow_start = true;
        let last_ack_seq_no = SequenceNumber::MAX_SEQ_NO;
        let loss = false;
        let last_dec_seq_no = SequenceNumber::MAX_SEQ_NO;
        let last_dec_period = Duration::from_micros(1);
        let loss_count = 0;
        let dec_random = 1;
        let avg_loss = 0;
        let dec_count = 1;
        let pkt_count = 0;
        Self {
            pkt_send_period,
            congestion_window,
            max_congestion_window,
            bandwidth,
            mss,
            cur_send_seq_no,
            recv_rate,
            rtt,
            rtt_var,
            ack_interval,
            rc_interval,
            last_rc_time,
            slow_start,
            last_ack_seq_no,
            loss,
            last_dec_seq_no,
            last_dec_period,
            loss_count,
            dec_random,
            avg_loss,
            dec_count,
            pkt_count
        }
    }
    pub fn on_ack(&mut self, ack: SequenceNumber) {
        let mut inc  ;
        let min_inc = 0.01;
        match self.last_rc_time.elapsed(){
            Ok(dur)=>{
                if dur<self.rc_interval{
                    return;
                }
            },
            Err(_)=>return
        }
        self.last_rc_time = SystemTime::now();
        if self.slow_start {
            self.congestion_window += (ack - self.last_ack_seq_no) as usize;
            self.last_ack_seq_no = ack;
            if self.congestion_window > self.max_congestion_window {
                self.slow_start = false;
                if self.recv_rate > 0 {
                    self.pkt_send_period = Duration::from_micros((100000 / self.recv_rate) as u64);
                } else {
                    self.pkt_send_period = Duration::from_micros(
                        (self.congestion_window
                            / ((self.rtt.as_micros() + self.rc_interval.as_micros()) as usize))
                            as u64,
                    );
                }
            }
        } else {
            self.congestion_window = self.recv_rate / 1000000
                * ((self.rtt.as_micros() + self.rc_interval.as_micros()) as usize)
                + 16;
        }
        if self.slow_start{
            return;
        }
        if self.loss{
            self.loss = false;
            return;
        }
        dbg!(self.bandwidth, self.pkt_send_period.as_micros());
        let mut b= self.bandwidth as isize-1000000/self.pkt_send_period.as_micros() as isize;
        if self.pkt_send_period>self.last_dec_period && self.bandwidth as isize/9<b{
            b = self.bandwidth as isize/9;
        }
        if b <=0{
            inc = min_inc;
        }else{
            inc = 10.0f64.powi(((b*self.mss as isize *8) as f64).log10().ceil() as i32)*0.0000015/self.mss as f64;
            if inc<min_inc{
                inc = min_inc
            }
        }
        let pkt_send_micros = self.pkt_send_period.as_micros() as f64;
        let rc_interval_micros = self.rc_interval.as_micros() as f64;
        self.pkt_send_period = Duration::from_micros(((pkt_send_micros*rc_interval_micros)/(pkt_send_micros*inc+rc_interval_micros)) as u64);

    }
    pub fn should_ack(&self)->bool{
        self.ack_interval>0 && self.ack_interval<self.pkt_count
    }
    pub fn next_ack(&mut self)->Duration{
        self.pkt_count=0;
        if self.ack_interval==0{
            SYN_INTERVAL
        }else{
            Duration::from_millis(self.ack_interval as u64) 
        }
    }
    pub fn inc_pkt_cnt(&mut self){
        self.pkt_count+=1;
    }
    pub fn long_poll(&self)->Duration{
        self.rtt+self.rtt_var*4
    }
    pub fn on_loss(&mut self, loss_start: SequenceNumber){
        if self.slow_start{
            self.slow_start = false;
            if self.recv_rate>0{
                self.pkt_send_period = Duration::from_micros(1000000/self.recv_rate as u64);
                return;
            }
            self.pkt_send_period = Duration::from_micros(self.congestion_window as u64 / (self.rtt + self.rc_interval).as_micros() as u64)
        }
        self.loss = true;
        if loss_start>self.last_dec_seq_no{
            self.last_dec_period = self.pkt_send_period;
            self.pkt_send_period = Duration::from_micros((self.pkt_send_period.as_millis() as f64 * 1.125) as u64);
            self.avg_loss =(self.avg_loss as f64 *0.875+self.loss_count as f64 *0.125) as usize;
            self.loss_count+=1;
            self.dec_count+=1;
            self.last_dec_seq_no = self.cur_send_seq_no;
            self.dec_random = utils::hash(self.last_dec_seq_no.0 as usize)/usize::MAX;
            if self.dec_random < 1{
                self.dec_random = 1;
            }
        }
    }
    pub fn on_timeout(&mut self) {
        if self.slow_start {
            self.slow_start = false;
            if self.recv_rate > 0 {
                self.pkt_send_period = Duration::from_micros(1000000 / self.recv_rate as u64);
            } else {
                self.pkt_send_period =  Duration::from_micros(self.congestion_window as u64 / (self.rtt + self.rc_interval).as_micros() as u64);
            }
        }
    }
    pub fn next_time(&self)->Duration{
        self.pkt_send_period
    }
    pub fn update_rtt(&mut self, rtt: u16){
        let rtt = rtt as i32;
        let mut self_rtt = self.rtt.as_millis() as i32;
        let mut self_rtt_var = self.rtt_var.as_millis() as i32;
        self_rtt_var = (self_rtt_var*3+(rtt-self_rtt).abs())>>2;
        self_rtt = (self_rtt*7+rtt)>>3;


        self.rtt = Duration::from_millis(self_rtt as u64);
        self.rtt_var = Duration::from_millis(self_rtt_var as u64);
    }

    pub fn update_recv_rate(&mut self, recv_rate: u16){
        self.recv_rate = (self.recv_rate*7+recv_rate as usize +4)>>3;
    }
    pub fn update_bandwidth(&mut self, bandwidth: u16){
        self.recv_rate = (self.bandwidth*7+bandwidth as usize+4)>>3;
    }
    pub fn rtt(&self)->(Duration,Duration){
        (self.rtt, self.rtt_var)
    }

}
