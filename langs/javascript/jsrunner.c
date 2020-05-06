/* File generated automatically by the QuickJS compiler. */

#include <stdlib.h>
#include <stdint.h>
#include <stdio.h>
#include <limits.h>
#include <errno.h>
#include <string.h>
#include <assert.h>
#include "quickjs-libc.h"
#include "quickjs.h"

#define wasm_export(name) __attribute__((export_name(#name)))

static char *io_buf = NULL;
// the size of io_buf. io_buf[io_buf_len] == '\0'
static size_t io_buf_len = 0;

wasm_export(__rr_prealloc) char *prealloc(size_t len)
{
  if (len > io_buf_len)
  {
    io_buf = realloc(io_buf, len + 1);
  }
  io_buf_len = len;
  io_buf[len] = '\0';
  return io_buf;
}

wasm_export(__rr_io_addr) char *_io_addr() { return io_buf; }

static void rr_init(void);
wasm_export(__rr_init) size_t robot_init()
{
  rr_init();
  return io_buf_len;
}

static void rr_runturn(void);
wasm_export(__rr_run_turn) size_t robot_run()
{
  rr_runturn();
  return io_buf_len;
}

// modified from js_load_file in quickjs-libc.c
static uint8_t *load_file(uint8_t *buf, size_t *pbuf_len, const char *filename)
{
  FILE *f;
  size_t buf_len;
  long lret;

  f = fopen(filename, "rb");
  if (!f)
    return NULL;
  if (fseek(f, 0, SEEK_END) < 0)
    goto fail;
  lret = ftell(f);
  if (lret < 0)
    goto fail;
  /* XXX: on Linux, ftell() return LONG_MAX for directories */
  if (lret == LONG_MAX)
  {
    errno = EISDIR;
    goto fail;
  }
  buf_len = lret;
  if (fseek(f, 0, SEEK_SET) < 0)
    goto fail;
  buf = realloc(buf, buf_len + 1);
  if (!buf)
    goto fail;
  if (fread(buf, 1, buf_len, f) != buf_len)
  {
    errno = EIO;
    free(buf);
  fail:
    fclose(f);
    return NULL;
  }
  buf[buf_len] = '\0';
  fclose(f);
  *pbuf_len = buf_len;
  return buf;
}

int main(int argc, char **argv)
{
  assert(argc > 1);
  io_buf = (char *)load_file((uint8_t *)io_buf, &io_buf_len, argv[1]);
  if (!io_buf)
  {
    fprintf(stderr, "failed to load input file '%s': %s", argv[1], strerror(errno));
    exit(1);
  }

  rr_init();

  printf("__rr_init:%.*s\n", (int)io_buf_len, io_buf);
  fflush(stdout);

  while (getline(&io_buf, &io_buf_len, stdin) != -1)
  {
    // printf("%.*s\n", (int)io_buf_len, io_buf);
    robot_run();
    printf("__rr_output:%.*s\n", (int)io_buf_len, io_buf);
    fflush(stdout);
  }
  return 0;
}

#define write_buf(sarr)       \
  prealloc(sizeof(sarr) - 1); \
  memcpy(io_buf, sarr, sizeof(sarr) - 1)
#define _INTERNAL_ERROR_JSON "{\"Err\":{\"InternalError\":null}}"
#define INTERNAL_ERROR write_buf(_INTERNAL_ERROR_JSON)

#define CHECK_ERR(ret)     \
  if (JS_IsException(ret)) \
  {                        \
    write_err();           \
    return;                \
  }

#define RETURN_IF_EXC(ret, before) \
  if (JS_IsException(ret))         \
  {                                \
    before;                        \
    return;                        \
  }

static JSRuntime *rt;
static JSContext *ctx;
static JSAtom format_err_atom;
static JSAtom main_atom;
static JSValue globalThis;

static void write_json_stringify(JSValue val)
{
  JSValue json = JS_JSONStringify(ctx, val, JS_UNDEFINED, JS_UNDEFINED);
  RETURN_IF_EXC(json, js_std_dump_error(ctx); exit(1));
  size_t plen = 0;
  const char *s = JS_ToCStringLen(ctx, &plen, json);
  JS_FreeValue(ctx, json);
  if (s == NULL)
  {
    js_std_dump_error(ctx);
    exit(1);
    return;
  }
  prealloc(plen);
  memcpy(io_buf, s, plen);
  JS_FreeCString(ctx, s);
}

static void write_err(int init_err)
{
  JSValue exc = JS_GetException(ctx);
  JSValue ret = JS_Invoke(ctx, globalThis, format_err_atom, init_err ? 3 : 2, (JSValue[]){exc, JS_TRUE, JS_TRUE});
  JS_FreeValue(ctx, exc);
  RETURN_IF_EXC(ret, js_std_dump_error(ctx); exit(1));
  write_json_stringify(ret);
  JS_FreeValue(ctx, ret);
}

static void rr_init(void)
{
  rt = JS_NewRuntime();
  ctx = JS_NewContext(rt);
  format_err_atom = JS_NewAtom(ctx, "__format_err");
  main_atom = JS_NewAtom(ctx, "__main");
  globalThis = JS_GetGlobalObject(ctx);

  extern const uint8_t qjsc_stdlib[];
  extern const uint32_t qjsc_stdlib_size;
  js_std_eval_binary(ctx, qjsc_stdlib, qjsc_stdlib_size, 0);

  // JS_Keys
  JSValue ret = JS_Eval(ctx, io_buf, io_buf_len, "<robot>", JS_EVAL_TYPE_GLOBAL);
  RETURN_IF_EXC(ret, write_err(1));
  // printf("AAA\n");
  write_buf("{\"Ok\":null}");
}

static void rr_runturn(void)
{
  JSValue input = JS_ParseJSON(ctx, io_buf, io_buf_len - 1, "input");
  RETURN_IF_EXC(input, js_std_dump_error(ctx); exit(1));
  JSValue ret = JS_Invoke(ctx, globalThis, main_atom, 1, &input);
  RETURN_IF_EXC(ret, js_std_dump_error(ctx); exit(1));
  write_json_stringify(ret);
  JS_FreeValue(ctx, ret);
}
