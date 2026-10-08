// Authored bounded external-reader experiment, not a translated parser.
// Loads whole RScript resources with the pinned Toolkit and prints the fields the
// U10 spot check compares (docs/u10-script-reader.md, "PC verification" step 3).
import com.google.gson.*;
import cwlib.resources.RScript;
import cwlib.types.SerializedResource;
import java.nio.file.*;
import java.util.*;

public class ScriptOracle {
    static final String PIN = "d6a2ef80b5a09999f08f23a803b516096055e5c9";
    static final long MAX_INPUT = 16L * 1024 * 1024;

    public static void main(String[] args) throws Exception {
        if (args.length < 2) throw new IllegalArgumentException("usage: ScriptOracle NEW-JSON SCRIPT...");
        Path out = Paths.get(args[0]);
        if (Files.exists(out)) throw new IllegalStateException("output exists");
        JsonArray rows = new JsonArray();
        for (int i = 1; i < args.length; i++) {
            Path p = Paths.get(args[i]);
            JsonObject row = new JsonObject();
            row.addProperty("file", p.getFileName().toString());
            try {
                if (Files.size(p) > MAX_INPUT) throw new IllegalStateException("input budget");
                SerializedResource res = new SerializedResource(Files.readAllBytes(p));
                RScript s = res.loadResource(RScript.class);
                row.addProperty("class", s.className);
                row.addProperty("super_class_guid", s.superClassScript == null || s.superClassScript.getGUID() == null
                    ? null : s.superClassScript.getGUID().getValue());
                row.addProperty("functions", s.functionDefinitions.size());
                row.addProperty("instructions", s.sharedBytecode.size());
                row.addProperty("status", "loaded");
            } catch (Exception e) {
                row.addProperty("status", "error");
                row.addProperty("reason", String.valueOf(e));
            }
            rows.add(row);
        }
        JsonObject doc = new JsonObject();
        doc.addProperty("toolkit_commit", PIN);
        doc.add("rows", rows);
        Files.writeString(out, new GsonBuilder().setPrettyPrinting().create().toJson(doc));
    }
}
