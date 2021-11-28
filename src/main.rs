extern crate daemonize;

use notify_rust::Notification;
use std::time::Duration;
use std::{fs::File, thread};

use battery::Battery;
use daemonize::Daemonize;

fn main() -> Result<(), battery::Error> {
    let stdout = File::create("/tmp/battrs_daemon.out").unwrap();
    let stderr = File::create("/tmp/battrs_daemon.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/tmp/battrs_daemon.pid") // Every method except `new` and `start`
        .chown_pid_file(true) // is optional, see `Daemonize` documentation
        .working_directory("/tmp") // for default behaviour.
        .user("nobody")
        .group("daemon") // Group name
        .group(2) // or group id.
        .umask(0o777) // Set umask, `0o027` by default.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr) // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }

    let manager = battery::Manager::new()?;

    let sleep_time = Duration::from_secs(60);

    // Percentages at which to generate a notification
    let notif_perc = vec![100, 75, 50, 25, 10, 5];

    // State of each battery
    let mut state: Vec<i16> = vec![];

    loop {
        manager
            .batteries()?
            .enumerate()
            .into_iter()
            .for_each(|(id, maybe_battery)| {
                let battery = maybe_battery.unwrap();
                let perc: i32 = bat_percent(&battery);

                if state.len() <= id {
                    let mut pos: i16 = 0;
                    while notif_perc[pos as usize] > perc {
                        pos += 1;
                    }
                    state.push(pos);
                };

                if notif_perc[state[id] as usize] > perc {
                    notif(battery, perc);
                    state[id] += 1;
                } else if state[id] != 0 && notif_perc[(state[id] - 1) as usize] <= perc {
                    notif(battery, perc);
                    state[id] -= 1;
                }
            });

        thread::sleep(sleep_time);
    }
}

fn bat_percent(battery: &Battery) -> i32 {
    (100.0 * (battery.energy().value / battery.energy_full().value)) as i32
}

fn notif(battery: Battery, perc: i32) {
    let str_state = format!("{}", battery.state());
    let time_to = if "discharging".eq(&str_state) {
        let to_empty = battery.time_to_empty();
        format!("empty: {:?} ", to_empty)
    } else {
        let to_full = battery.time_to_full();
        format!("full charge: {:?} ", to_full)
    };

    let bat_txt = format!("State: {:?} ({:?}%) \nTime to {}", str_state, perc, time_to);

    Notification::new()
        .summary("Battery level")
        .body(&bat_txt)
        .show()
        .unwrap();
}
