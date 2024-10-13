(in-package :cl-user)
(defpackage :siren.bin
  (:use :cl)
  (:local-nicknames
   (#:v #:org.shirakumo.verbose))
  (:import-from :clingon)
  (:export
   :main
   :buildapp-main))
(in-package :siren.bin)
