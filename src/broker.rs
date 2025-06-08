use crate::types::Order;
use std::mem;
use zmq;

/// This `Broker` holds any per-strategy settings (e.g. commission/margin)
/// plus a ZMQ PUSH socket.  When a strategy decides to place an order,
/// it simply calls `self.broker.send(order)`.
pub struct Broker {
    /// You can store whatever per-strategy fees/parameters you need here:
    pub commission_fee: f64,
    pub margin_ratio: f64,

    /// Internally, one ZMQ PUSH socket.  Each strategy will own exactly one.
    socket: zmq::Socket,
}

impl Broker {
    /// Create a new broker.  `ctx` is a cloned zmq::Context; `order_uri`
    /// is the same PUSH‐endpoint that your engine expects.
    pub fn new(ctx: &zmq::Context, order_uri: &str, commission_fee: f64, margin_ratio: f64) -> Self {
        let sock = ctx.socket(zmq::PUSH).expect("Failed to create PUSH socket");
        // unlimited hwm so we never block
        sock.set_sndhwm(0).expect("Failed to settting");
        // linger = 0 so close doesn’t block
        sock.set_linger(0).expect("Failed to settting");
        sock.connect(order_uri).expect("Failed to connect PUSH to order_uri");

        Broker {
            commission_fee,
            margin_ratio,
            socket: sock,
        }
    }

    /// Whenever a strategy wants to place an order, it calls this.
    /// We serialize the raw `Order` struct (including any C-ABI padding)
    /// and send it over ZMQ in one go.
    pub fn send(&self, order: &Order) {
        // If you need to apply commission or adjust volume using margin_ratio,
        // you can do that here (e.g. modify fields of `order` or log them).
        //
        // For now, we just shove the raw bytes onto the wire:
        let bytes: &[u8] = unsafe {
            let ptr = order as *const Order as *const u8;
            std::slice::from_raw_parts(ptr, mem::size_of::<Order>())
        };
        if let Err(e) = self.socket.send(bytes, 0) {
            eprintln!("Broker failed to send order: {:?}", e);
        }
    }

    pub fn charge() -> f64 {
        1e-4
    }

    pub fn buy() {}

    pub fn sell() {}

    pub fn sell_short() {}

    pub fn buy_cover() {}
}
