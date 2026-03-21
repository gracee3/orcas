CC ?= cc
CFLAGS ?= -O2 -Wall -Wextra -pedantic

.PHONY: test clean

test: hello
	./tests/test.sh

hello: main.c
	$(CC) $(CFLAGS) main.c -o hello

clean:
	rm -f hello
