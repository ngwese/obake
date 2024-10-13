(in-package :siren.bin)

(defun top-level/options ()
  (list
   (clingon:make-option :enum
			:long-name "log-level"
			:description "logging level"
			:env-vars '("LOG_LEVEL")
			:items '(("info" . :info)
				 ("warn" . :warn)
				 ("error" . :error)
				 ("debug" . :debug)
				 ("trace" . :trace))
			:initial-value "info"
			:key :log-level)))

(defun top-level/handler (cmd)
  (let ((level (clingon:getopt cmd :log-level)))
    (progn
      ;; TODO: validate this is the right way to set overall log level
      (setf (v:repl-level) level)
      (v:debug :app "starting")
      ;;(sleep 2)
      ;;(sb-impl::toplevel-repl nil)
      (let ((swank-port (swank:create-server :port 4005 :dont-close t)))
	(v:info :app "swank port: ~a" swank-port))
      (v:debug :app "stopping"))))
  
(defun top-level/command ()
  (clingon:make-command :name "siren"
			:version "0.1.0"
			:description "up up up"
			:handler #'top-level/handler
			:options (top-level/options)))

(defun main ()
  "The main entrypoint of siren app"
  (let ((app (top-level/command)))
      (clingon:run app)))

(defun buildapp-main (argv)
  "The main entrypoint for buildapp"
  (let ((app (top-level/command)))
    (clingon:run app (rest argv))))


(defmethod v:format-message ((stream stream) (message v:message))
  (let ((elapsed (float (/ (get-internal-real-time)
			   internal-time-units-per-second))))
    (format stream "~&[~,7f][~5,a] ~{<~a>~}: ~a"
	    elapsed
	    (v:level message)
	    (v:categories message)
	    (v:content message))))
