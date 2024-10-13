(defpackage :siren-bin-system
  (:use :cl :asdf))
(in-package :siren-bin-system)

(defsystem "siren.bin"
  :name "siren.bin"
  :long-name "siren.bin"
  :depends-on (:clingon
	       :verbose
	       :cl-collider
	       :cl-pattern
	       :swank
	       )
  :components ((:module "bin"
		:serial t
		:pathname #p"src/bin/"
		:components ((:file "package")
			     (:file "main"))))
  :build-operation "program-op"
  :build-pathname "siren"
  :entry-point "siren.bin:main")

