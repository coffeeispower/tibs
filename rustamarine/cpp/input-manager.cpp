#include <aquamarine/input/Input.hpp>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <hyprutils/memory/SharedPtr.hpp>
#include <print>
#include <rustamarine/internal/rustamarine.hpp>
#include <string>

using namespace rustamarine;
using namespace Aquamarine;
// Implementation of Mouse
Mouse::Mouse(SP<Aquamarine::IPointer> pointer, InputManager *inputManager)
		: pointer(pointer), inputManager(inputManager) {
	std::println("New mouse: {}", pointer->getName());
	// Listen for relative mouse movement
	onRelativeMoveListenerListener =
			pointer->events.move.registerListener([this](std::any event) {
				auto relEvent = std::any_cast<Aquamarine::IPointer::SMoveEvent>(event);
				this->inputManager->mouseDeltaX += relEvent.delta.x;
				this->inputManager->mouseDeltaY += relEvent.delta.y;
				this->inputManager->mouseAbsoluteX += relEvent.delta.x;
				this->inputManager->mouseAbsoluteY += relEvent.delta.y;
				if (this->inputManager->rmar->screens.empty())
					return;
				auto screen = this->inputManager->rmar->screens[0];
			});
	// Listen for absolute mouse movement (warp)
	onWarpListener =
			pointer->events.warp.registerListener([this](std::any event) {
				auto warpEvent = std::any_cast<Aquamarine::IPointer::SWarpEvent>(event);
				if (!this->inputManager->rmar->screens.empty()) {
					auto screen = this->inputManager->rmar->screens[0];
					auto width = static_cast<double>(rmarScreenGetWidth(screen.get()));
					auto height = static_cast<double>(rmarScreenGetHeight(screen.get()));
					auto oldAbsolutePositionX = this->inputManager->mouseAbsoluteX;
					auto oldAbsolutePositionY = this->inputManager->mouseAbsoluteY;
					this->inputManager->mouseAbsoluteX = warpEvent.absolute.x * width;
					this->inputManager->mouseAbsoluteY = warpEvent.absolute.y * height;
					this->inputManager->mouseDeltaX +=
							warpEvent.absolute.x - this->inputManager->mouseAbsoluteX;
					this->inputManager->mouseDeltaY +=
							warpEvent.absolute.y - this->inputManager->mouseAbsoluteY;
				}
			});
	onScrollListener = pointer->events.axis.registerListener([this](std::any d) {
    auto scrollEvent = std::any_cast<Aquamarine::IPointer::SAxisEvent>(d);
    switch (scrollEvent.axis) {
	   	case IPointer::AQ_POINTER_AXIS_VERTICAL:
					this->inputManager->mouseScrollY += scrollEvent.delta;
					break;
	    case IPointer::AQ_POINTER_AXIS_HORIZONTAL:
					this->inputManager->mouseScrollX += scrollEvent.delta;
					break;
    }
	});
	// Listen for pointer disconnect
	onDisconnectListener =
			pointer->events.destroy.registerListener([this](std::any) {
				auto pointerPtr = this->pointer.get();
				std::erase_if(this->inputManager->mouses,
											[pointerPtr](const auto &mouse) {
												return mouse->pointer.get() == pointerPtr;
											});
			});
}

// Implementation of Keyboard
Keyboard::Keyboard(SP<Aquamarine::IKeyboard> keyboard,
									 InputManager *inputManager)
		: keyboard(keyboard), inputManager(inputManager) {
	// Initialize xkbcommon
	if (!initXkb()) {
		// Failed to initialize XKB
		fprintf(stderr, "Failed to initialize XKB for keyboard\n");
		return;
	}

	// Listen for key updates
	this->onKeyUpdateListener =
			keyboard->events.key.registerListener([this](std::any event) {
				if (xkbState == nullptr)
					return;
				auto keyEvent = std::any_cast<Aquamarine::IKeyboard::SKeyEvent>(event);
				auto keycode = keyEvent.key; // Raw keycode from libinput
				auto evdev_keycode =
						keycode + 8; // Adjusted for xkbcommon (evdev uses +8 offset)
				auto pressed = keyEvent.pressed;
				auto timeMs = keyEvent.timeMs;

				// Update xkb state for modifiers and other state tracking
				xkb_state_update_key(xkbState, evdev_keycode,
														 pressed ? XKB_KEY_DOWN : XKB_KEY_UP);

				// Get proper keysym from xkb state
				xkb_keysym_t keysym =
						xkb_state_key_get_one_sym(xkbState, evdev_keycode);

				// Update key state tracking (separate from character input)
				auto &state = this->keystates[keysym];
				bool prevDown = state.down;
				state.down = pressed;
				state.justChanged = (state.down != prevDown);
				state.shouldTypeChar = state.shouldTypeChar && state.justChanged;
				state.repeating = false;
				state.stateChangedTimestamp = timeMs;

				// Handle key event with xkbcommon for character conversion
				if (pressed)
					handleKeyEvent(keysym);
			});

	// Listen for keyboard disconnect
	onDisconnectListener =
			keyboard->events.destroy.registerListener([this](std::any) {
				// Remove this keyboard from inputManager->keyboards
				auto kbdPtr = this->keyboard.get();
				std::erase_if(this->inputManager->keyboards, [kbdPtr](const auto &kbd) {
					return kbd->keyboard.get() == kbdPtr;
				});
			});
}

Keyboard::~Keyboard() { cleanupXkb(); }

bool Keyboard::initXkb() {
	// Create XKB context
	xkbContext = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
	if (!xkbContext) {
		return false;
	}
	struct xkb_rule_names rules = {.rules = "evdev", .model = "", .layout = getenv("RUSTAMARINE_KB_LAYOUT")};

	xkbKeymap = xkb_keymap_new_from_names(xkbContext, &rules,
																				XKB_KEYMAP_COMPILE_NO_FLAGS);

	if (!xkbKeymap) {
		cleanupXkb();
		return false;
	}

	xkbState = xkb_state_new(xkbKeymap);
	if (!xkbState) {
		cleanupXkb();
		return false;
	}

	const char *locale = setlocale(LC_CTYPE, nullptr);
	xkbComposeTable = xkb_compose_table_new_from_locale(
			xkbContext, locale, XKB_COMPOSE_COMPILE_NO_FLAGS);
	if (xkbComposeTable) {
		xkbComposeState =
				xkb_compose_state_new(xkbComposeTable, XKB_COMPOSE_STATE_NO_FLAGS);
	}

	return true;
}

void Keyboard::cleanupXkb() {
	if (xkbComposeState) {
		xkb_compose_state_unref(xkbComposeState);
		xkbComposeState = nullptr;
	}

	if (xkbComposeTable) {
		xkb_compose_table_unref(xkbComposeTable);
		xkbComposeTable = nullptr;
	}

	if (xkbState) {
		xkb_state_unref(xkbState);
		xkbState = nullptr;
	}

	if (xkbKeymap) {
		xkb_keymap_unref(xkbKeymap);
		xkbKeymap = nullptr;
	}

	if (xkbContext) {
		xkb_context_unref(xkbContext);
		xkbContext = nullptr;
	}
}

std::string Keyboard::keysymToUtf8(xkb_keysym_t keysym) {
	if (!xkbState) {
		return "";
	}

	char buffer[64];
	int len = xkb_keysym_to_utf8(keysym, buffer, sizeof(buffer));

	if (len <= 0) {
		return "";
	}

	buffer[len] = '\0';
	return std::string(buffer);
}

void Keyboard::handleKeyEvent(xkb_keysym_t keysym) {

	// Skip non-printable keysyms
	if (keysym == XKB_KEY_NoSymbol) {
		return;
	}

	// Check if any control or alt modifiers are active
	bool ctrl_active =
			xkb_state_mod_name_is_active(xkbState, XKB_MOD_NAME_CTRL,
																	 XKB_STATE_MODS_EFFECTIVE) == 1;
	bool alt_active = xkb_state_mod_name_is_active(xkbState, XKB_MOD_NAME_ALT,
																								 XKB_STATE_MODS_EFFECTIVE) == 1;

	// Skip key combinations with modifiers that shouldn't produce text
	if (ctrl_active || alt_active) {
		return;
	}

	// Skip non-printable keys
	if (!xkb_keysym_to_utf8(keysym, nullptr, 0)) {
		return;
	}

	// Handle compose sequence if available
	if (xkbComposeState) {
		xkb_compose_feed_result result =
				xkb_compose_state_feed(xkbComposeState, keysym);

		if (result == XKB_COMPOSE_FEED_ACCEPTED) {
			xkb_compose_status status = xkb_compose_state_get_status(xkbComposeState);

			if (status == XKB_COMPOSE_COMPOSED) {
				// Composition complete, get the composed string
				char buffer[64];
				int len =
						xkb_compose_state_get_utf8(xkbComposeState, buffer, sizeof(buffer));

				if (len > 0) {
					buffer[len] = '\0';
					inputManager->currentFrameUtf8Input += std::string(buffer);
				}

				xkb_compose_state_reset(xkbComposeState);
				return;
			} else if (status == XKB_COMPOSE_NOTHING ||
								 status == XKB_COMPOSE_CANCELLED) {
				xkb_compose_state_reset(xkbComposeState);
			} else {
				// Still composing, wait for more input
				return;
			}
		}
	}

	// No compose or compose not applicable, convert directly to UTF-8
	std::string utf8 = keysymToUtf8(keysym);
	if (!utf8.empty()) {
		inputManager->currentFrameUtf8Input += utf8;
	}
}

// Implementation of InputManager
InputManager::InputManager(SP<Rustamarine> rmar) : rmar(rmar) {
	// Listen for new keyboards
	onNewKeyboardListener = rmar->backend->events.newKeyboard.registerListener(
			[rmar](std::any event) {
				auto keyboard = std::any_cast<SP<Aquamarine::IKeyboard>>(event);
				rmar->inputManager.keyboards.emplace_back(
						Hyprutils::Memory::makeShared<Keyboard>(keyboard,
																										&rmar->inputManager));
			});
	// Listen for new mice
	onNewMouseListener =
			rmar->backend->events.newPointer.registerListener([rmar](std::any event) {
				auto pointer = std::any_cast<SP<Aquamarine::IPointer>>(event);
				rmar->inputManager.mouses.emplace_back(
						Hyprutils::Memory::makeShared<Mouse>(pointer, &rmar->inputManager));
			});
}

const std::string &InputManager::getUtf8Characters() {
	return currentFrameUtf8Input;
}

// xkbcommon modifier names
#define XKB_MOD_NAME_SHIFT "Shift"
#define XKB_MOD_NAME_CAPS "Lock"
#define XKB_MOD_NAME_CTRL "Control"
#define XKB_MOD_NAME_ALT "Mod1"
#define XKB_MOD_NAME_NUM "Mod2"
#define XKB_MOD_NAME_LOGO "Mod4"

// Helper function to get current time in milliseconds since epoch
static inline uint32_t getCurrentTimeMs() {
	using namespace std::chrono;
	return static_cast<uint32_t>(
			duration_cast<milliseconds>(steady_clock::now().time_since_epoch())
					.count());
}
void InputManager::onPollEvents() {

	// Reset mouse delta for the new frame
	mouseDeltaX = 0;
	mouseDeltaY = 0;
	mouseScrollX = 0;
	mouseScrollY = 0;
	// Clear character input for the new frame
	currentFrameUtf8Input.clear();
	for (auto &kb : this->keyboards) {
		// Reset justChanged for all keys and mouse buttons
		for (auto &[_, state] : kb->keystates) {
			// Handle repeating and shouldTypeChar logic
			uint32_t now = getCurrentTimeMs();
			if (!state.down) {

				// Key is not held, reset repeat state
				state.repeating = false;
				state.shouldTypeChar = false;
				state.justChanged = false;
				continue;
			}

			// shouldTypeChar should only last 1 frame
			if (state.shouldTypeChar) {
				// If it was just set to true, leave it for this frame
				// It will be set to false on the next frame unless triggered again
				state.shouldTypeChar = false;
			} else if (state.justChanged) {
				// Key was just pressed, start repeat timer
				state.repeating = false;
				state.lastTypedCharTimestamp = now;
				state.shouldTypeChar = true;
			} else {
				// Key is held, check for repeat
				if (!state.repeating) {
					// Start repeating after initial delay (e.g., 400ms)
					if (now - state.stateChangedTimestamp >= 400) {
						state.repeating = true;
						state.lastTypedCharTimestamp = now;
						state.shouldTypeChar = true;
					}
				} else {
					// Already repeating, fire every 16ms
					if (now - state.lastTypedCharTimestamp >= 16) {
						state.shouldTypeChar = true;
						state.lastTypedCharTimestamp = now;
					}
				}
			}
			state.justChanged = false;
		}
	}
	for (auto &mouse : this->mouses)
		for (auto &[_, state] : mouse->mouseButtonStates) {
			state.justChanged = false;
		}
}
bool rmarIsKeyDown(Rustamarine *rmar, uint32_t key) {
	if (!rmar)
		return false;
	for (auto &kb : rmar->inputManager.keyboards) {
		auto it = kb->keystates.find(key);
		if (it != kb->keystates.end() && it->second.down)
			return true;
	}
	return false;
}

bool rmarIsKeyPressed(Rustamarine *rmar, uint32_t key) {
	if (!rmar)
		return false;
	for (auto &kb : rmar->inputManager.keyboards) {
		auto it = kb->keystates.find(key);
		if (it != kb->keystates.end() && it->second.down) {
			// If key is down in this keyboard, justChanged must be true in all such
			// keyboards
			for (auto &kb2 : rmar->inputManager.keyboards) {
				auto it2 = kb2->keystates.find(key);
				if (it2 != kb2->keystates.end() && it2->second.down &&
						!it2->second.justChanged)
					return false;
			}
			return true;
		}
	}
	return false;
}

bool rmarIsKeyReleased(Rustamarine *rmar, uint32_t key) {
	if (!rmar)
		return false;
	for (auto &kb : rmar->inputManager.keyboards) {
		auto it = kb->keystates.find(key);
		if (it != kb->keystates.end() && !it->second.down) {
			// This keyboard has the key up, now check all keyboards with down ==
			// false
			for (auto &kb2 : rmar->inputManager.keyboards) {
				auto it2 = kb2->keystates.find(key);
				if (it2 != kb2->keystates.end() && !it2->second.down &&
						!it2->second.justChanged)
					return false;
			}
			return true;
		}
	}
	return false;
}

bool rmarShouldTypeKey(Rustamarine *rmar, uint32_t key) {
	if (!rmar)
		return false;
	for (auto &kb : rmar->inputManager.keyboards) {
		auto it = kb->keystates.find(key);
		if (it != kb->keystates.end() && it->second.shouldTypeChar)
			return true;
	}
	return false;
}

bool rmarIsMouseButtonDown(Rustamarine *rmar, uint32_t button) {
	if (!rmar)
		return false;
	for (auto &mouse : rmar->inputManager.mouses) {
		auto it = mouse->mouseButtonStates.find(button);
		if (it != mouse->mouseButtonStates.end() && it->second.down)
			return true;
	}
	return false;
}

bool rmarIsMouseButtonPressed(Rustamarine *rmar, uint32_t button) {
	if (!rmar)
		return false;
	for (auto &mouse : rmar->inputManager.mouses) {
		auto it = mouse->mouseButtonStates.find(button);
		if (it != mouse->mouseButtonStates.end() && it->second.down &&
				it->second.justChanged)
			return true;
	}
	return false;
}

bool rmarIsMouseButtonReleased(Rustamarine *rmar, uint32_t button) {
	if (!rmar)
		return false;
	for (auto &mouse : rmar->inputManager.mouses) {
		auto it = mouse->mouseButtonStates.find(button);
		if (it != mouse->mouseButtonStates.end() && !it->second.down &&
				it->second.justChanged)
			return true;
	}
	return false;
}

int rmarGetMouseX(Rustamarine *rmar) {
	if (!rmar)
		return 0;
	return static_cast<int>(rmar->inputManager.mouseAbsoluteX);
}

int rmarGetMouseY(Rustamarine *rmar) {
	if (!rmar)
		return 0;
	return static_cast<int>(rmar->inputManager.mouseAbsoluteY);
}

int rmarGetMouseDeltaX(Rustamarine *rmar) {
	if (!rmar)
		return 0;
	return static_cast<int>(rmar->inputManager.mouseDeltaX);
}

int rmarGetMouseDeltaY(Rustamarine *rmar) {
	if (!rmar)
		return 0;
	return static_cast<int>(rmar->inputManager.mouseDeltaY);
}

int rmarGetMouseScrollX(Rustamarine *rmar) {
	return rmar->inputManager.mouseScrollX;
}

int rmarGetMouseScrollY(Rustamarine *rmar) {
	return rmar->inputManager.mouseScrollY;
}

const char *rmarGetTypedCharacters(Rustamarine *rmar) {
	if (!rmar) {
		return "";
	}
	return rmar->inputManager.getUtf8Characters().c_str();
}

void rmarSetMouseX(Rustamarine *rmar, int x) {
	if (!rmar)
		return;
	double prev = rmar->inputManager.mouseAbsoluteX;
	rmar->inputManager.mouseAbsoluteX = static_cast<double>(x);
	rmar->inputManager.mouseDeltaX += rmar->inputManager.mouseAbsoluteX - prev;
}

void rmarSetMouseY(Rustamarine *rmar, int y) {
	if (!rmar)
		return;
	double prev = rmar->inputManager.mouseAbsoluteY;
	rmar->inputManager.mouseAbsoluteY = static_cast<double>(y);
	rmar->inputManager.mouseDeltaY += rmar->inputManager.mouseAbsoluteY - prev;
}
