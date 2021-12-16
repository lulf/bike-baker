#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use defmt_rtt as _;
use panic_probe as _;

use drogue_device::{
    actors::button::{Button as ButtonActor, ButtonPressed},
    bsp::boards::nrf52::microbit::*,
    domain::led::matrix::Frame,
    traits::button::Button,
    traits::led::LedMatrix,
    Actor, ActorContext, Address, Board, Inbox,
};

use lsm303agr::{interface::I2cInterface, mode::MagOneShot, AccelOutputDataRate, Lsm303agr};

use core::future::Future;

use embassy::time::{Duration, Timer};
use embassy_nrf::{interrupt, peripherals::TWISPI0, twim, Peripherals};

#[embassy::main]
async fn main(spawner: embassy::executor::Spawner, p: Peripherals) {
    let board = Microbit::new(p);

    // LED matrix is our progress bar, but we can add additional
    static LED_MATRIX: ActorContext<LedMatrixActor> = ActorContext::new();
    let matrix = LED_MATRIX.mount(spawner, LedMatrixActor::new(board.led_matrix, None));

    static PROGRESS_BAR: ActorContext<ProgressBar> = ActorContext::new();
    let progress_bar = PROGRESS_BAR.mount(spawner, ProgressBar::new(matrix));

    // Game logic depends on progress bar and acts on input from baker and chaos monkey
    static GAME: ActorContext<Game> = ActorContext::new();
    let game = GAME.mount(spawner, Game::new(progress_bar));

    // Chaos monkey is based on the accelerometer, so hand the i2c bus to this actor which is the only one using it.
    static CHAOS_MONKEY: ActorContext<ChaosMonkey> = ActorContext::new();
    let config = twim::Config::default();
    let irq = interrupt::take!(SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0);
    let twi = twim::Twim::new(board.twispi0, irq, board.p0_16, board.p0_08, config);
    CHAOS_MONKEY.mount(spawner, ChaosMonkey::new(game, twi));

    // Actor for button 'A' that will progress baking
    static BUTTON_BAKER: ActorContext<ButtonBaker<ButtonA>> = ActorContext::new();
    BUTTON_BAKER.mount(spawner, ButtonBaker::new(board.button_a, game));

    // Actor for button 'B' that will restart
    static RESTART: ActorContext<ButtonActor<ButtonB, ButtonPressed<Game>>> = ActorContext::new();
    RESTART.mount(
        spawner,
        ButtonActor::new(board.button_b, ButtonPressed(game, GameEvent::Restart)),
    );
}

// Progress goes from 0 to 100
pub type Progress = usize;
const PROGRESS_INITIAL: Progress = 0;
const PROGRESS_DONE: Progress = 100;

pub struct Game {
    progress_bar: Address<ProgressBar>,
    progress: Progress,
}

#[derive(Clone)]
pub enum GameEvent {
    Restart,
    Progressed,
    ChaosInflicted,
}

impl Game {
    pub fn new(progress_bar: Address<ProgressBar>) -> Self {
        Self {
            progress_bar,
            progress: 0,
        }
    }
}

impl Actor for Game {
    type Message<'m> = GameEvent;

    type OnMountFuture<'m, M>
    where
        M: 'm,
    = impl Future<Output = ()> + 'm;
    fn on_mount<'m, M>(
        &'m mut self,
        _: Address<Self>,
        inbox: &'m mut M,
    ) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {
            defmt::info!("Game started!");
            loop {
                if let Some(mut m) = inbox.next().await {
                    match *m.message() {
                        GameEvent::Progressed => {
                            self.progress += core::cmp::min(1, PROGRESS_DONE - self.progress);
                            defmt::info!("Progress! Current progress: {}", self.progress);
                        }
                        GameEvent::ChaosInflicted => {
                            defmt::info!("Chaos! Current progress: {}", self.progress);
                            self.progress -= core::cmp::min(self.progress, 5);
                        }
                        GameEvent::Restart => {
                            defmt::info!("Restarting game!");
                            self.progress = PROGRESS_INITIAL;
                        }
                    }
                    let _ = self.progress_bar.notify(self.progress);
                    if self.progress >= PROGRESS_DONE {
                        defmt::info!("Baking done!");
                        self.progress = PROGRESS_INITIAL;
                    }
                }
            }
        }
    }
}

pub struct ProgressBar {
    display: Address<LedMatrixActor>,
}

impl ProgressBar {
    pub fn new(display: Address<LedMatrixActor>) -> Self {
        Self { display }
    }
}

impl Actor for ProgressBar {
    type Message<'m> = Progress;
    type OnMountFuture<'m, M>
    where
        M: 'm,
    = impl Future<Output = ()> + 'm;
    fn on_mount<'m, M>(
        &'m mut self,
        _: Address<Self>,
        inbox: &'m mut M,
    ) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {
            let mut mat: Frame<5, 5> = Frame::empty();
            loop {
                if let Some(mut m) = inbox.next().await {
                    let progress = *m.message();
                    // Normalize to LED matrix scale

                    let progress = progress as usize / (PROGRESS_DONE / 25);
                    for i in 0..progress {
                        let (x, y) = (i % 5, (i / 5) % 5);
                        mat.set(x, y);
                    }
                    for i in progress..25 {
                        let (x, y) = (i % 5, (i / 5) % 5);
                        mat.unset(x, y);
                    }
                    let _ = self.display.apply(&mat).await;
                }
            }
        }
    }
}

pub struct ChaosMonkey {
    game: Address<Game>,
    lsm: Lsm303agr<I2cInterface<twim::Twim<'static, TWISPI0>>, MagOneShot>,
}

impl ChaosMonkey {
    pub fn new(game: Address<Game>, i2c: twim::Twim<'static, TWISPI0>) -> Self {
        let lsm = Lsm303agr::new_with_i2c(i2c);
        Self { game, lsm }
    }
}

impl Actor for ChaosMonkey {
    type OnMountFuture<'m, M>
    where
        M: 'm,
    = impl Future<Output = ()> + 'm;
    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {
            self.lsm.init().unwrap();
            self.lsm.set_accel_odr(AccelOutputDataRate::Hz50).unwrap();
            let (mut x, mut y, mut z);
            loop {
                if self.lsm.accel_status().unwrap().xyz_new_data {
                    let data = self.lsm.accel_data().unwrap();
                    x = data.x;
                    y = data.y;
                    z = data.z;
                    break;
                }
            }
            defmt::info!("Chaos monkey initialized");
            const MAX_OFFSET: i32 = 100;
            loop {
                if self.lsm.accel_status().unwrap().xyz_new_data {
                    let data = self.lsm.accel_data().unwrap();
                    if x + MAX_OFFSET < data.x || x - MAX_OFFSET > data.x {
                        let _ = self.game.notify(GameEvent::ChaosInflicted);
                    }
                    if y + MAX_OFFSET < data.y || y - MAX_OFFSET > data.y {
                        let _ = self.game.notify(GameEvent::ChaosInflicted);
                    }
                    if z + MAX_OFFSET < data.z || z - MAX_OFFSET > data.z {
                        let _ = self.game.notify(GameEvent::ChaosInflicted);
                    }
                    x = data.x;
                    y = data.y;
                    z = data.z;
                    defmt::info!("Acceleration: x {} y {} z {}", data.x, data.y, data.z);
                }
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    }
}

pub struct ButtonBaker<B>
where
    B: Button,
{
    game: Address<Game>,
    button: B,
}

impl<B> ButtonBaker<B>
where
    B: Button,
{
    pub fn new(button: B, game: Address<Game>) -> Self {
        Self { button, game }
    }
}

impl<B> Actor for ButtonBaker<B>
where
    B: Button,
{
    type OnMountFuture<'m, M>
    where
        M: 'm,
        Self: 'm,
    = impl Future<Output = ()> + 'm;
    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {
            loop {
                self.button.wait_pressed().await;
                let _ = self.game.notify(GameEvent::Progressed);
            }
        }
    }
}
