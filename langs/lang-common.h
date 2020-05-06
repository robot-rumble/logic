#ifndef _LANG_COMMON_H
#define _LANG_COMMON_H

#include <stdio.h>
#include <errno.h>
#include <assert.h>
#include <string.h>
#include <stdint.h>
#include <limits.h>

#define wasm_export(name) __attribute__((export_name(#name)))

static char *io_buf = NULL;
// the size of io_buf. io_buf[io_buf_len] == '\0'
static size_t io_buf_len = 0;

// the user must define this function after including this lang-common.h
static void rr_init(void);
// the user must define this function after including this lang-common.h
static void rr_runturn(void);

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

wasm_export(__rr_init) size_t robot_init()
{
  rr_init();
  return io_buf_len;
}

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

#endif
