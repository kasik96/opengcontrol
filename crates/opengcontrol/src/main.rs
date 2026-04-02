mod cli;
mod config;
mod context;
mod output;
mod permissions;
mod spinner;
mod style;
mod tui;

use clap::Parser;
use cli::{Cli, Commands};
use context::{enumerate_physical_devices, DeviceContext};
use hidpp_core::HidppError;

fn main() {
    if let Err(e) = run() {
        eprintln!("\n  {}  {}\n", style::g_red(style::SYM_FAIL), e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    // Commands that don't need a device connection
    match &cli.command {
        Commands::List => return cli::device::handle_list(cli.output),
        Commands::Doctor(args) => return cli::doctor::handle_doctor(args),
        _ => {}
    }

    let output = cli.output;

    // Enumerate physical devices (one entry per mouse)
    let devices = enumerate_physical_devices().map_err(|e| e.to_string())?;

    if devices.is_empty() {
        let e = HidppError::NoDeviceFound;
        return Err(format!(
            "{e}\n\n  {}  Run {} to see connected devices and diagnose issues.",
            style::dim(style::SYM_ARROW),
            style::bold("opengcontrol doctor")
        ));
    }

    // For TUI: may need an in-TUI device selector when multiple devices are present
    if matches!(cli.command, Commands::Tui(_)) {
        return if let Some(path) = cli.device.as_deref() {
            // Specific device requested
            let ctx = open_device(Some(path), output)?;
            cli::tui::handle_tui(ctx)
        } else if devices.len() == 1 {
            let ctx = open_device(Some(&devices[0].path), output)?;
            cli::tui::handle_tui(ctx)
        } else {
            // Multiple devices: show ratatui selector screen
            tui::run_with_selection(devices, output).map_err(|e| e.to_string())
        };
    }

    // For all other commands: determine which device to open
    let path = if let Some(path) = cli.device.as_deref() {
        path.to_string()
    } else if devices.len() == 1 {
        devices[0].path.clone()
    } else {
        // Interactive numbered selector
        cli::selector::select(&devices)?
    };

    let ctx = open_device(Some(&path), output)?;

    // Show device header in human mode before running the subcommand
    if matches!(output, output::OutputFormat::Human) {
        print_device_header(&ctx);
    }

    match cli.command {
        Commands::Info => handle_info(&ctx, output),
        Commands::Dpi(args) => cli::dpi::handle_dpi(&ctx, &args.command, output),
        Commands::Polling(args) => cli::polling::handle_polling(&ctx, &args.command, output),
        Commands::Profile(args) => cli::profile::handle_profile(&ctx, &args.command, output),
        Commands::Tui(_) | Commands::List | Commands::Doctor(_) => unreachable!(),
    }
}

fn open_device(path: Option<&str>, output: output::OutputFormat) -> Result<DeviceContext, String> {
    DeviceContext::open(path, output).map_err(|e| match &e {
        HidppError::PermissionDenied => {
            permissions::print_permission_guidance();
            e.to_string()
        }
        _ => e.to_string(),
    })
}

fn print_device_header(ctx: &DeviceContext) {
    let info = ctx.device_info();
    println!();
    println!(
        "  {}  {}  {}",
        style::g_cyan(style::SYM_DEVICE),
        style::g_cyan_bold(info.name),
        style::dim(&format!("{:04X}:{:04X}", info.vid, info.pid))
    );
}

fn handle_info(ctx: &DeviceContext, fmt: output::OutputFormat) -> Result<(), String> {
    use hidpp_core::features::{AdjustableDpi, OnboardProfiles, PollingRate};
    use output::{json, OutputFormat};

    let info = ctx.device_info();

    // Gather all values (best-effort; show "–" on error)
    let dpi = AdjustableDpi::get_dpi(ctx.device(), 0)
        .map(|d| style::g_cyan_bold(&d.to_string()))
        .unwrap_or_else(|_| style::dim("–"));

    let rate = PollingRate::get_rate_hz(ctx.device())
        .map(|r| style::g_cyan_bold(&format!("{r} Hz")))
        .unwrap_or_else(|_| style::dim("–"));

    let profile = OnboardProfiles::get_active_profile(ctx.device())
        .map(|p| style::g_cyan_bold(&p.to_string()))
        .unwrap_or_else(|_| style::dim("–"));

    match fmt {
        OutputFormat::Human => {
            style::print_rule();
            style::print_kv("DPI", &dpi);
            style::print_kv("Polling rate", &rate);
            style::print_kv("Active profile", &profile);
            println!();
        }
        OutputFormat::Json => {
            // Re-fetch raw values for JSON
            let dpi_raw = AdjustableDpi::get_dpi(ctx.device(), 0).ok();
            let rate_raw = PollingRate::get_rate_hz(ctx.device()).ok();
            let profile_raw = OnboardProfiles::get_active_profile(ctx.device()).ok();
            json::print_json(&serde_json::json!({
                "device": info.name,
                "vid": format!("{:04X}", info.vid),
                "pid": format!("{:04X}", info.pid),
                "dpi": dpi_raw,
                "polling_rate_hz": rate_raw,
                "active_profile": profile_raw,
            }));
        }
    }

    Ok(())
}
