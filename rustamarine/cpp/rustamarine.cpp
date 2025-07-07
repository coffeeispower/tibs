#include "rustamarine.h"
#include "aquamarine/backend/Backend.hpp"
#include <aquamarine/output/Output.hpp>
#include <cstdlib>
#include <cstring>
#include <hyprutils/memory/SharedPtr.hpp>
#include <iostream>
#include <poll.h>
#include <rustamarine/internal/opengl.hpp>
#include <rustamarine/internal/rustamarine.hpp>

bool enableDebugLogs = !strcmp(std::getenv("AQ_ENABLE_DEBUG_LOGS") ? std::getenv("AQ_ENABLE_DEBUG_LOGS") : "", "1");
bool enableTraceLogs = !strcmp(std::getenv("AQ_ENABLE_TRACE_LOGS") ? std::getenv("AQ_ENABLE_TRACE_LOGS") : "", "1");
void aqLog(Aquamarine::eBackendLogLevel level, std::string msg) {
	if (level == Aquamarine::AQ_LOG_DEBUG && !enableDebugLogs)
		return;
	if (level == Aquamarine::AQ_LOG_TRACE && !enableTraceLogs)
		return;
	std::cout << "[AQ] [" << aqLevelToString(level) << "] " << msg << "\n";
}
std::vector<Aquamarine::SBackendImplementationOptions> getBackendsList() {
	std::vector<Aquamarine::SBackendImplementationOptions> implementations;
	Aquamarine::SBackendImplementationOptions backendOptions;
	backendOptions.backendType = Aquamarine::eBackendType::AQ_BACKEND_WAYLAND;
	backendOptions.backendRequestMode =
			Aquamarine::eBackendRequestMode::AQ_BACKEND_REQUEST_FALLBACK;
	implementations.emplace_back(backendOptions);
	backendOptions.backendType = Aquamarine::eBackendType::AQ_BACKEND_HEADLESS;
	backendOptions.backendRequestMode =
			Aquamarine::eBackendRequestMode::AQ_BACKEND_REQUEST_MANDATORY;
	implementations.emplace_back(backendOptions);
	backendOptions.backendType = Aquamarine::eBackendType::AQ_BACKEND_DRM;
	backendOptions.backendRequestMode =
			Aquamarine::eBackendRequestMode::AQ_BACKEND_REQUEST_IF_AVAILABLE;
	implementations.emplace_back(backendOptions);
	return implementations;
}

void setupEventListeners(SP<Rustamarine> rmar) {
	rmar->listeners.newOutputListener =
			rmar->backend->events.newOutput.registerListener(
					[rmar](std::any uncastedOutput) {
						auto output =
								std::any_cast<SP<Aquamarine::IOutput>>(uncastedOutput);

						rmar->screens.push_back(createScreenFromOutput(rmar, output));
					});

}

Rustamarine *rmarInitialize() {
	setup_segfault_handler();
	Aquamarine::SBackendOptions options;
	options.logFunction = aqLog;
	auto implementations = getBackendsList();
	auto aqBackend = Aquamarine::CBackend::create(implementations, options);
	SP<Rustamarine> rmar(
			new Rustamarine{.backend = aqBackend, .screens = {}, .listeners = {}});
	setupEventListeners(rmar);
	rmar->inputManager = rustamarine::InputManager(rmar);
	if (!rmar->backend->start())
		panic("Failed to start aquamarine backend");
	initializeOpenGL(rmar);

	rmar.impl_->inc();
	return rmar.get();
}

void rmarPollEvents(struct Rustamarine *self) {
	for (auto &screen : self->screens) {
		screen->isVBlank = false;
	}

	self->inputManager.onFrameEnd();
	auto pollFDs = self->backend->getPollFDs();
	std::vector<pollfd> fds;
	for (const auto &pfd : pollFDs) {
		fds.push_back({pfd->fd, POLLIN, 0});
	}
	int ret = poll(fds.data(), fds.size(), -1);
	if (ret > 0) {
		for (size_t i = 0; i < fds.size(); ++i) {
			if (fds[i].revents & POLLIN) {
				auto fd = fds[i].fd;
				auto it = std::find_if(pollFDs.begin(), pollFDs.end(),
															 [fd](const auto &pfd) { return pfd->fd == fd; });
				if (it == pollFDs.end())
					continue;
				it.base()->get()->onSignal();
			}
		}
	}
}
void rmarTearDown(struct Rustamarine *self) {
	tearDownOpenGL(&self->openGLContext);
	delete self;
}
struct RustamarineScreens rmarGetScreens(struct Rustamarine *self) {
	struct RustamarineScreens result;
	result.count = self->screens.size();
	result.screens = nullptr;
	if (result.count > 0) {
		result.screens = (struct RustamarineScreen **)malloc(
				sizeof(struct RustamarineScreen *) * result.count);
		for (size_t i = 0; i < result.count; ++i) {
			result.screens[i] = self->screens[i].get();
		}
	}
	return result;
}
void rmarFreeScreens(struct RustamarineScreens screens) {
	free(screens.screens);
}
struct Rustamarine *rmarFromScreen(struct RustamarineScreen *screen) {
	return screen->rustamarine.get();
}
bool rmarIsDRM(struct Rustamarine* rmar) {
	return rmar->backend->hasSession();
}

void rmarGoToTTY(struct Rustamarine *rmar, uint16_t tty) {
	rmar->backend->session->switchVT(tty);
}
bool rmarIsOnOriginalTTY(struct Rustamarine* rmar) {
	if(!rmarIsDRM(rmar)) return true;
	return rmar->backend->session->active;
}
