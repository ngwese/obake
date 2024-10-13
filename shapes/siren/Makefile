LISP ?= sbcl

bin:
	${LISP} --eval '(push :verbose-no-init *features*)' \
		--eval '(ql:quickload :siren.bin)' \
		--eval '(asdf:make :siren.bin)' \
		--eval '(quit)'

.PHONY: bin
