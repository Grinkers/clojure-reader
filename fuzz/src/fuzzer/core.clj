(ns fuzzer.core
  (:import [foo.bar Rust])
  (:require [clojure.test-clojure.generators :as cgen]
            [clojure.edn :as cljedn])
  (:gen-class))

(defn- init! []
  (clojure.lang.RT/loadLibrary "clojure_reader"))

(defn -main
  [& args]
  (init!)
  (dotimes [_ (if (first args) (Integer/parseInt (first args)) 4209042)]
    (let [source (pr-str (cgen/ednable))
          ; clojure-reader doesn't preserve order on hashmaps or sets. For now we just use clojure's
          ; read-string to make sure the round trip is valid edn.
          clojure-reader (-> (Rust/roundtrip source) cljedn/read-string)
          edn (cljedn/read-string source)]
      (when-not (= edn clojure-reader)
        (println "source:")
        (prn (pr-str source))
        (println "clojure.edn::")
        (prn (pr-str edn))
        (println "clojure-reader:")
        (prn (pr-str clojure-reader))
        (throw (new Exception "Not Equal")))))
  (println "All done!"))
