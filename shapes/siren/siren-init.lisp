(defpackage #:siren-user
  (:use #:cl #:cl-collider #:cl-pattern)
  (:local-nicknames
   (#:v #:org.shirakumo.verbose)))

;; (setf sb-int:*repl-prompt-function*
;;      (lambda (stream)
;; 	(format stream "~s> " (package-name *package*))))

(in-package :siren-user)
(named-readtables:in-readtable :sc)

;; please check *sc-synth-program*, *sc-plugin-paths*, *sc-synthdefs-path*
;; if you have different path then set to
;;
;; (setf *sc-synth-program* "/path/to/scsynth")
;; (setf *sc-plugin-paths* (list "/path/to/plugin_path" "/path/to/extension_plugin_path"))
;; (setf *sc-synthdefs-path* "/path/to/synthdefs_path")

;; `*s*` defines the server for the entire session
;; functions may use it internally.
(v:info :init "starting server")
(setf *s* (make-external-server "localhost" :port 48800))
(server-boot *s*)

;; in Linux, maybe you need to call this function
#+linux
(progn
  (v:info :init "connecting jack")
  (jack-connect))
