pub struct ContainerIter<'a> {
    buf: &'a [f64],
    idx: usize,
    remaining: usize,
}

impl<'a> Iterator for ContainerIter<'a> {
    type Item = &'a f64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            let item = &self.buf[self.idx];
            self.idx = (self.idx + 1) % self.buf.len();
            self.remaining -= 1;
            Some(item)
        }
    }
}

pub struct Container {
    buf: Vec<f64>,
    head_idx: usize,
    tail_idx: usize,
}

impl Container {
    pub fn new(n: usize) -> Self {
        Self {
            buf: vec![f64::NAN; n],
            head_idx: 0,
            tail_idx: 0,
        }
    }

    pub fn update(&mut self, new_val: f64) -> (f64, f64) {
        self.tail_idx = self.head_idx;
        self.buf[self.tail_idx] = new_val;
        self.head_idx = (self.head_idx + 1) % self.buf.len();

        // current_old, current_new after updated
        (self.buf[self.head_idx], self.buf[self.tail_idx])
    }

    pub fn step(&mut self) -> (f64, f64) {
        self.tail_idx = self.head_idx;
        self.head_idx = (self.head_idx + 1) % self.buf.len();

        // current_old, current_new after updated
        (self.buf[self.head_idx], self.buf[self.tail_idx])
    }

    pub fn get(&self, idx: usize) -> f64 {
        // idx=0 is head; idx=n-1 is tail
        self.buf[(self.head_idx + idx) % self.buf.len()]
    }

    pub fn head(&self) -> f64 {
        self.buf[self.head_idx]
    }

    pub fn tail(&self) -> f64 {
        self.buf[self.tail_idx]
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn iter(&self) -> ContainerIter<'_> {
        ContainerIter {
            buf: &self.buf,
            idx: self.head_idx,
            remaining: self.buf.len(),
        }
    }
}

pub struct Sum {
    pub container: Container,
    nan_count: usize,
    sum: f64,
}

impl Sum {
    pub fn new(n: usize) -> Self {
        Self {
            container: Container::new(n),
            nan_count: n,
            sum: 0.0,
        }
    }

    pub fn update(&mut self, new_val: f64) -> f64 {
        let old_val = self.container.head();
        self.container.update(new_val);

        if old_val.is_finite() {
            self.sum -= old_val;
        } else {
            self.nan_count -= 1;
        }

        if new_val.is_finite() {
            self.sum += new_val;
        } else {
            self.nan_count += 1;
        }

        if self.nan_count > 0 { f64::NAN } else { self.sum }
    }
}

pub struct WeightedSum {
    container: Container,
    weights: Vec<f64>,
}

impl WeightedSum {
    pub fn new(weights: Vec<f64>) -> Self {
        let n = weights.len();
        Self {
            container: Container::new(n),
            weights,
        }
    }

    pub fn update(&mut self, new_val: f64) -> f64 {
        self.container.update(new_val);

        self.weights
            .iter()
            .zip(self.container.iter())
            .map(|(&weight, &value)| weight * value)
            .sum()
    }
}

// no NAN rolling average
pub struct Mean {
    container: Container,
    nan_count: usize,
    sum: f64,
}

impl Mean {
    pub fn new(n: usize) -> Self {
        Self {
            container: Container::new(n),
            nan_count: n,
            sum: 0.0,
        }
    }

    pub fn update(&mut self, new_val: f64) -> f64 {
        let old_val = self.container.head();
        self.container.update(new_val);

        if old_val.is_finite() {
            self.sum -= old_val;
        } else {
            self.nan_count -= 1;
        }

        if new_val.is_finite() {
            self.sum += new_val;
        } else {
            self.nan_count += 1;
        }

        if self.nan_count > 0 {
            self.sum / (self.container.len() - self.nan_count) as f64
        } else {
            self.sum / self.container.len() as f64
        }
    }
}

pub struct StDev {
    sumer: Sum,
    sq_sumer: Sum,
    n: usize,
}

impl StDev {
    pub fn new(n: usize) -> Self {
        Self {
            sumer: Sum::new(n),
            sq_sumer: Sum::new(n),
            n,
        }
    }

    pub fn update(&mut self, new_val: f64) -> f64 {
        let sum = self.sumer.update(new_val);
        let sq_sum = self.sq_sumer.update(new_val * new_val);

        let variance = (sq_sum - sum * sum / self.n as f64) / (self.n as f64 - 1.0);
        variance.sqrt()
    }
}
