(defproject fuzzer "0.1.0-SNAPSHOT"
  :description "FIXME: write description"
  :url "http://example.com/FIXME"
  :license {:name "EPL-2.0 OR GPL-2.0-or-later WITH Classpath-exception-2.0"
            :url "https://www.eclipse.org/legal/epl-2.0/"}
  :dependencies [[org.clojure/clojure "1.11.1"]
                 [org.clojure/data.generators "1.0.0"]]
  :main ^:skip-aot fuzzer.core
  :target-path "target/%s"
  :jvm-opts ["-Djava.library.path=./resources"]
  :java-source-paths ["src-java"]
  :resources-paths ["resources"]
  :profiles {:uberjar {:aot :all
                       :jvm-opts ["-Dclojure.compiler.direct-linking=true"
                                  "-Dclojure.spec.skip-macros=true"]}})
