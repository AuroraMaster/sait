use conduit::{debug, debug_info, error, implement, info, Err, Result};
use ruma::events::room::message::RoomMessageEventContent;
use tokio::time::{sleep, Duration};

/// Possibly spawn the terminal console at startup if configured.
#[implement(super::Service)]
pub(super) async fn console_auto_start(&self) {
	#[cfg(feature = "console")]
	if self.services.server.config.admin_console_automatic {
		// Allow more of the startup sequence to execute before spawning
		tokio::task::yield_now().await;
		self.console.start().await;
	}
}

/// Shutdown the console when the admin worker terminates.
#[implement(super::Service)]
pub(super) async fn console_auto_stop(&self) {
	#[cfg(feature = "console")]
	self.console.close().await;
}

/// Execute admin commands after startup
#[implement(super::Service)]
pub(super) async fn startup_execute(&self) -> Result<()> {
	// List of comamnds to execute
	let commands = &self.services.server.config.admin_execute;

	// Determine if we're running in smoketest-mode which will change some behaviors
	let smoketest = self.services.server.config.test.contains("smoke");

	// When true, errors are ignored and startup continues.
	let errors = !smoketest && self.services.server.config.admin_execute_errors_ignore;

	//TODO: remove this after run-states are broadcast
	sleep(Duration::from_millis(500)).await;

	for (i, command) in commands.iter().enumerate() {
		if let Err(e) = self.startup_execute_command(i, command.clone()).await {
			if !errors {
				return Err(e);
			}
		}

		tokio::task::yield_now().await;
	}

	// The smoketest functionality is placed here for now and simply initiates
	// shutdown after all commands have executed.
	if smoketest {
		debug_info!("Smoketest mode. All commands complete. Shutting down now...");
		self.services
			.server
			.shutdown()
			.inspect_err(error::inspect_log)
			.expect("Error shutting down from smoketest");
	}

	Ok(())
}

/// Execute one admin command after startup
#[implement(super::Service)]
async fn startup_execute_command(&self, i: usize, command: String) -> Result<()> {
	debug!("Startup command #{i}: executing {command:?}");

	match self.command_in_place(command, None).await {
		Ok(Some(output)) => Self::startup_command_output(i, &output),
		Err(output) => Self::startup_command_error(i, &output),
		Ok(None) => {
			info!("Startup command #{i} completed (no output).");
			Ok(())
		},
	}
}

#[cfg(feature = "console")]
#[implement(super::Service)]
fn startup_command_output(i: usize, content: &RoomMessageEventContent) -> Result<()> {
	debug_info!("Startup command #{i} completed:");
	super::console::print(content.body());
	Ok(())
}

#[cfg(feature = "console")]
#[implement(super::Service)]
fn startup_command_error(i: usize, content: &RoomMessageEventContent) -> Result<()> {
	super::console::print_err(content.body());
	Err!(debug_error!("Startup command #{i} failed."))
}

#[cfg(not(feature = "console"))]
#[implement(super::Service)]
fn startup_command_output(i: usize, content: &RoomMessageEventContent) -> Result<()> {
	info!("Startup command #{i} completed:\n{:#?}", content.body());
	Ok(())
}

#[cfg(not(feature = "console"))]
#[implement(super::Service)]
fn startup_command_error(i: usize, content: &RoomMessageEventContent) -> Result<()> {
	Err!(error!("Startup command #{i} failed:\n{:#?}", content.body()))
}
