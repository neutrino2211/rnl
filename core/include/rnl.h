#ifndef RNL_H
#define RNL_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * C-compatible element factory vtable
 *
 * Each element type (button, box, text, etc.) provides an instance of this struct
 * that defines how to create and manipulate that element.
 */
typedef struct rnl_RnlElementFactory {
  /**
   * Unique element name (e.g., "button", "box", "text-field")
   */
  const char *name;
  /**
   * Create a new instance of this element
   */
  void *(*create)(void);
  /**
   * Set an attribute/prop on the widget
   */
  void (*set_attribute)(void*, const char*, const char*);
  /**
   * Set a callback attribute (e.g., onClick)
   */
  void (*set_callback)(void*, const char*, void*);
  /**
   * Append a child widget
   */
  void (*append_child)(void*, void*);
  /**
   * Insert child before a reference widget
   */
  void (*insert_before)(void*, void*, void*);
  /**
   * Remove a child widget
   */
  void (*remove_child)(void*, void*);
  /**
   * Destroy the widget and free resources
   */
  void (*destroy)(void*);
} rnl_RnlElementFactory;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Main entry point - called by platform's main()
 *
 * # Safety
 * - bundle_path must be a valid C string or NULL
 * - argv must be an array of argc valid C strings
 */
int rnl_main(const char *bundle_path, int argc, char **argv);

/**
 * Execute a JS bundle (called by platform after window is ready)
 *
 * # Safety
 * - bundle must be a valid C string
 */
int rnl_execute_bundle(const char *bundle);

/**
 * Log a message from native code (routed to JS console)
 *
 * # Safety
 * - level and message must be valid C strings
 */
void rnl_log(const char *level, const char *message);

/**
 * Report an error from native code (will throw in JS)
 *
 * # Safety
 * - message must be a valid C string
 */
void rnl_error(const char *message);

/**
 * Implementation of callback invocation (called by bridge.rs)
 *
 * # Safety
 * This is only called from rnl_invoke_callback in bridge.rs
 */
int rnl_invoke_callback_impl(uint64_t callback_id);

/**
 * C API: Register an element factory
 *
 * # Safety
 * - factory must point to a valid RnlElementFactory
 * - The factory must remain valid for the program's lifetime
 */
void rnl_register_element(const struct rnl_RnlElementFactory *factory);

/**
 * C API: Invoke a JS callback from native code
 *
 * # Safety
 * - callback must be a valid callback ID (cast to pointer)
 * - event_json must be a valid C string (currently unused)
 */
void rnl_invoke_callback(void *callback, const char *event_json);

/**
 * C API: Set the root container handle (called by platform)
 *
 * # Safety
 * - root must be a valid platform widget pointer
 */
void rnl_set_root_container(void *root);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* RNL_H */
