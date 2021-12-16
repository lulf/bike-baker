#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use defmt_rtt as _;
use panic_probe as _;

use drogue_device::{
    actors::button::{Button, ButtonPressed},
    actors::led::matrix::{AnimationEffect, MatrixCommand},
    bsp::boards::nrf52::microbit::*,
    Actor, ActorContext, Address, Board, Inbox,
};

use core::future::Future;

use embassy::time::Duration;
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
    static BAKER: ActorContext<Button<ButtonA, ButtonPressed<Game>>> = ActorContext::new();
    BAKER.mount(
        spawner,
        Button::new(board.button_a, ButtonPressed(game, GameEvent::Progressed)),
    );

    // Actor for button 'B' that will restart
    static RESTART: ActorContext<Button<ButtonB, ButtonPressed<Game>>> = ActorContext::new();
    RESTART.mount(
        spawner,
        Button::new(board.button_b, ButtonPressed(game, GameEvent::Restart)),
    );
}

// Progress goes from 0 to 100
pub type Progress = u8;
const INITIAL: Progress = 0;
const DONE: Progress = 100;

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
    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {}
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
    type OnMountFuture<'m, M>
    where
        M: 'm,
    = impl Future<Output = ()> + 'm;
    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {}
    }
}

pub struct ChaosMonkey {
    game: Address<Game>,
    i2c: twim::Twim<'static, TWISPI0>,
}

impl ChaosMonkey {
    pub fn new(game: Address<Game>, i2c: twim::Twim<'static, TWISPI0>) -> Self {
        Self { game, i2c }
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
        async move {}
    }
}

pub struct Baker {
    game: Address<Game>,
}

impl Baker {
    pub fn new(game: Address<Game>) -> Self {
        Self { game }
    }
}

impl Actor for Baker {
    type OnMountFuture<'m, M>
    where
        M: 'm,
    = impl Future<Output = ()> + 'm;
    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {}
    }
}
