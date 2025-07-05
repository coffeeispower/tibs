cmake_minimum_required(VERSION 3.16)
project(rustamarine-cpp LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 23)

# Include dirs (podem vir do Rust depois via ENV vars se necessário)
# Automatically collect all .cpp, .c, .hpp, .h files, ignoring the build directory
file(GLOB_RECURSE RUSTAMARINE_CPP_SOURCES
    RELATIVE ${CMAKE_CURRENT_SOURCE_DIR}
    *.cpp
    *.c
    *.hpp
    *.h
)

# Remove files in the build directory from the list
list(FILTER RUSTAMARINE_CPP_SOURCES EXCLUDE REGEX "^build/")

add_library(rustamarine-cpp STATIC
    ${RUSTAMARINE_CPP_SOURCES}
)

# Se precisares de pkg-config, podes usar isso
find_package(PkgConfig REQUIRED)
pkg_check_modules(AQUAMARINE REQUIRED aquamarine)
pkg_check_modules(HYPRUTILS REQUIRED hyprutils)
pkg_check_modules(PIXMAN REQUIRED pixman-1)
pkg_check_modules(LIBBACKTRACE REQUIRED libbacktrace)
pkg_check_modules(LIBDRM REQUIRED libdrm)
pkg_check_modules(LIBGBM REQUIRED gbm)
pkg_check_modules(LIBUNWIND REQUIRED libunwind)

target_compile_options(rustamarine-cpp PUBLIC
  ${AQUAMARINE_CFLAGS}
  ${HYPRUTILS_CFLAGS}
  ${PIXMAN_CFLAGS}
  ${LIBBACKTRACE_CFLAGS}
  ${LIBDRM_CFLAGS}
  ${LIBUNWIND_CFLAGS}
)

target_include_directories(rustamarine-cpp PUBLIC
  headers
  vendor/glad/include
  ${PIXMAN_INCLUDE_DIRS}
  ${LIBBACKTRACE_INCLUDE_DIRS}
)

target_link_libraries(rustamarine-cpp
  PUBLIC
  ${AQUAMARINE_LIBRARIES}
  ${HYPRUTILS_LIBRARIES}
  ${PIXMAN_LIBRARIES}
  ${LIBBACKTRACE_LIBRARIES}
  ${LIBDRM_LIBRARIES}
  ${LIBGBM_LIBRARIES}
  ${LIBUNWIND_LIBRARIES}
  EGL GLESv2
)


# Instalar biblioteca
install(TARGETS rustamarine-cpp
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
    RUNTIME DESTINATION bin
)
