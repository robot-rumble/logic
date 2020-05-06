#include <stdlib.h>
#include <stdint.h>
#include <stdio.h>
#include <limits.h>
#include <errno.h>
#include <string.h>
#include "../lang-common.h"
#include "quickjs-libc.h"
#include "quickjs.h"

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
