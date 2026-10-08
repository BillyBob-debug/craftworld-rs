// Authored bounded external-reader experiment (D119), not a translated parser.
// Runs the pinned Toolkit/cwlib RPlan reader and its Thing array locally and projects a
// per-Thing summary for parity with lbp-level::plan. Never shipped, never a product
// dependency. Provenance: docs/d119-level-backdrop.md.
import com.google.gson.*;
import cwlib.enums.Part;
import cwlib.io.serializer.Serializer;
import cwlib.resources.RPlan;
import cwlib.structs.things.Thing;
import cwlib.structs.things.parts.*;
import cwlib.types.SerializedResource;
import cwlib.types.data.GUID;
import cwlib.types.data.ResourceDescriptor;
import org.joml.Matrix4f;
import java.io.*;
import java.lang.reflect.Field;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

public class PlanThingOracle {
    static final String PIN="d6a2ef80b5a09999f08f23a803b516096055e5c9";
    static final String SCOPE="plnb_toolkit_plan_things_v1";
    static final int MAX_STORED=16*1024*1024, MAX_SAMPLES=256;
    static final long MAX_RUN_BYTES=256L*1024*1024;

    static Map<String,Object> row(Object... f) {
        Map<String,Object> m=new TreeMap<>();for(int i=0;i<f.length;i+=2)m.put((String)f[i],f[i+1]);return m;
    }
    static String guid(GUID g){return g==null?null:String.format(Locale.ROOT,"%08x",g.getValue());}
    static String descGuid(ResourceDescriptor d){return d==null||!d.isGUID()?null:guid(d.getGUID());}
    static List<Float> m44(Matrix4f m){if(m==null)return null;float[] f=m.get(new float[16]);List<Float> l=new ArrayList<>();for(float v:f)l.add(v);return l;}
    static List<String> parts(Thing t) {
        List<String> l=new ArrayList<>();
        for(Part p:Part.values())if(p.getSerializable()!=null&&t.hasPart(p))l.add(p.name());
        return l;
    }
    static Field visited;
    static {
        try{visited=Serializer.class.getDeclaredField("visited");visited.setAccessible(true);}
        catch(ReflectiveOperationException e){throw new ExceptionInInitializerError(e);}
    }
    static String sha1(byte[] b)throws Exception {return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-1").digest(b));}

    @SuppressWarnings("unchecked")
    static Map<String,Object> inspect(byte[] raw)throws Exception {
        if(raw.length<12||raw[0]!='P'||raw[1]!='L'||raw[2]!='N'||raw[3]!='b')throw new IOException("only ordinary binary PLNb");
        SerializedResource res=new SerializedResource(raw);
        RPlan plan=res.loadResource(RPlan.class);
        // Same construction as RPlan.getThings, without the post-load fixup.
        Serializer s=new Serializer(plan.thingData,plan.revision,plan.compressionFlags);
        Thing[] roots=s.array(null,Thing.class,true);
        TreeMap<Integer,Object> tags=new TreeMap<>((Map<Integer,Object>)visited.get(s));
        List<Object> things=new ArrayList<>();
        for(Object o:tags.values()) {
            if(!(o instanceof Thing))continue;
            Thing t=(Thing)o;
            PPos pos=t.getPart(Part.POS);PRenderMesh rm=t.getPart(Part.RENDER_MESH);PLevelSettings ls=t.getPart(Part.LEVEL_SETTINGS);
            things.add(row("uid",t.UID,"plan_guid",guid(t.planGUID),"parts",parts(t),
                "world",pos==null?null:m44(pos.worldPosition),
                "mesh",rm==null?null:descGuid(rm.mesh),"anim",rm==null?null:descGuid(rm.anim),
                "bones",rm==null||rm.boneThings==null?null:rm.boneThings.length,
                "backdrop_mesh",ls==null?null:descGuid(ls.backdropMesh)));
        }
        return row("head",res.getRevision().getHead(),"thing_data_bytes",plan.thingData.length,
            "consumed_thing_bytes",s.getOffset(),"roots",roots.length,"things",things);
    }

    static boolean sameArchive(String sourcePath,Path archive) {
        String name=archive.getFileName().toString().toLowerCase(Locale.ROOT);
        return sourcePath.startsWith("USRDIR/")&&sourcePath.toLowerCase(Locale.ROOT).endsWith("/"+name);
    }

    static void run(String[] args)throws Exception {
        if(args.length<3)throw new IllegalArgumentException("MANIFEST NEW-JSON ARCHIVE...");
        Path output=Path.of(args[1]).toAbsolutePath();
        if(!output.toString().endsWith(".json")||Files.exists(output)||
            !output.getParent().toRealPath().startsWith(Path.of("local-analysis").toRealPath()))throw new IOException("new ignored JSON required");
        JsonObject manifest=JsonParser.parseString(Files.readString(Path.of(args[0]))).getAsJsonObject();
        if(!manifest.get("toolkit_commit").getAsString().equals(PIN)||!manifest.get("scope").getAsString().equals(SCOPE))
            throw new IOException("manifest pin/scope differs");
        JsonArray samples=manifest.getAsJsonArray("samples");if(samples.size()>MAX_SAMPLES)throw new IOException("sample budget");
        List<Path> archives=new ArrayList<>();for(int i=2;i<args.length;i++)archives.add(Path.of(args[i]));
        Map<Path,RandomAccessFile> open=new HashMap<>();
        List<Object> rows=new ArrayList<>();long total=0;Map<String,Long> status=new TreeMap<>();
        try {
            for(JsonElement item:samples) {
                JsonObject q=item.getAsJsonObject();String sha=q.get("sha1").getAsString();
                Map<String,Object> r=row("sha1",sha,"guid",q.get("guid").getAsString(),"map_path",q.get("map_path").getAsString());
                rows.add(r);
                try {
                    String src=q.get("source_path").getAsString();
                    List<Path> match=new ArrayList<>();for(Path a:archives)if(sameArchive(src,a))match.add(a);
                    if(match.size()!=1)throw new IOException("archive not supplied or ambiguous: "+src);
                    RandomAccessFile f=open.get(match.get(0));
                    if(f==null){f=new RandomAccessFile(match.get(0).toFile(),"r");open.put(match.get(0),f);}
                    long at=q.get("offset").getAsLong(),size=q.get("size").getAsLong();
                    if(at<0||size<12||size>MAX_STORED||at>f.length()-size||total+size>MAX_RUN_BYTES)throw new IOException("source/read budget");
                    byte[] raw=new byte[(int)size];f.seek(at);f.readFully(raw);total+=size;
                    if(!sha1(raw).equals(sha))throw new IOException("identifier mismatch");
                    r.putAll(inspect(raw));
                    r.put("status",r.get("consumed_thing_bytes").equals(r.get("thing_data_bytes"))?"parsed_exact":"parsed_trailing_bytes");
                }catch(StackOverflowError e){r.putAll(row("status","failed","reason","StackOverflowError"));}
                catch(Exception e){
                    String msg=e.getMessage();if(msg!=null&&msg.length()>300)msg=msg.substring(0,300);
                    r.putAll(row("status",e instanceof IOException?"rejected":"failed","reason",e.getClass().getSimpleName()+": "+msg));
                }
                status.merge((String)r.get("status"),1L,Long::sum);
            }
        } finally {for(RandomAccessFile f:open.values())f.close();}
        Files.writeString(output,new GsonBuilder().serializeNulls().serializeSpecialFloatingPointValues().setPrettyPrinting().create().toJson(row(
            "schema_version",1,"toolkit_commit",PIN,"scope",SCOPE,"sample_bytes",total,"status_counts",status,"plans",rows)),
            StandardOpenOption.CREATE_NEW);
        System.out.println("Plan Thing oracle: "+rows.size()+" plans, "+total+" archived bytes, "+status);
    }
    public static void main(String[] args)throws Exception {
        Throwable[] failure=new Throwable[1];
        Thread t=new Thread(null,()->{try{run(args);}catch(Throwable e){failure[0]=e;}},"plan-thing-oracle",1L<<30);
        t.start();t.join();
        if(failure[0]!=null){failure[0].printStackTrace();System.exit(1);}
    }
}
