#pragma once


#ifdef __cplusplus
#include <stdint.h>
extern "C" {
#endif
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

struct RustamarineScreen;
struct Rustamarine;

struct Rustamarine *rmarInitialize();
void *rmarGetProcAddress(struct Rustamarine *self, const char *procName);
void rmarPollEvents(struct Rustamarine *self);
void rmarTearDown(struct Rustamarine *self);

struct RustamarineScreens {
	struct RustamarineScreen **screens;
	size_t count;
};

struct RustamarineScreens rmarGetScreens(struct Rustamarine *self);
void rmarFreeScreens(struct RustamarineScreens screens);
bool rmarIsVBlank(const struct RustamarineScreen *self);
void rmarUseScreen(struct RustamarineScreen *screen);
void rmarSwapBuffers(struct RustamarineScreen *self);
struct Rustamarine *rmarFromScreen(struct RustamarineScreen *screen);
void rmarScreenSetOnRender(struct RustamarineScreen *screen,
													 void (*callback)(void *, struct RustamarineScreen *),
													 void *context);
unsigned int rmarScreenGetWidth(const struct RustamarineScreen *screen);
unsigned int rmarScreenGetHeight(const struct RustamarineScreen *screen);
float rmarScreenGetRefreshRate(const struct RustamarineScreen *screen);
const char *rmarScreenGetName(const struct RustamarineScreen *screen);
bool rmarScreenIsEnabled(const struct RustamarineScreen *screen);
void rmarScreenSetEnabled(const struct RustamarineScreen *screen, bool isEnabled);

bool rmarIsKeyDown(struct Rustamarine* rmar, uint32_t key);
bool rmarIsKeyPressed(struct Rustamarine* rmar, uint32_t key);
bool rmarShouldTypeKey(struct Rustamarine* rmar, uint32_t key);
bool rmarIsKeyReleased(struct Rustamarine* rmar, uint32_t key);

bool rmarIsMouseButtonDown(struct Rustamarine* rmar, uint32_t button);
bool rmarIsMouseButtonPressed(struct Rustamarine* rmar, uint32_t button);
bool rmarIsMouseButtonReleased(struct Rustamarine* rmar, uint32_t button);

int rmarGetMouseX(struct Rustamarine* rmar);
int rmarGetMouseY(struct Rustamarine* rmar);
void rmarSetMouseX(struct Rustamarine* rmar, int x);
void rmarSetMouseY(struct Rustamarine* rmar, int y);
int rmarGetMouseDeltaX(struct Rustamarine* rmar);
int rmarGetMouseDeltaY(struct Rustamarine* rmar);
double rmarGetMouseScrollX(struct Rustamarine* rmar);
double rmarGetMouseScrollY(struct Rustamarine* rmar);


const char* rmarGetTypedCharacters(struct Rustamarine* rmar);

bool rmarIsDRM(struct Rustamarine* rmar);
void rmarGoToTTY(struct Rustamarine* rmar, uint16_t tty);
bool rmarIsOnOriginalTTY(struct Rustamarine* rmar);
#ifdef __cplusplus
}
#endif
