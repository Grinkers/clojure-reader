package foo.bar;

public class Rust {
    private static native String roundtripRust(String edn);

    public static String roundtrip(String edn) throws java.io.IOException {
        String output = roundtripRust(edn);
        return output;
    }
}
