// Authored bounded external-reader adapter, not an application material parser.
// Pinned reader/spec provenance and applicability: docs/material-prefix.md.
// Only clear ordinary PS3 GMTb; never deserialize RGfxMaterial or emit code bytes.
import com.google.gson.*;
import cwlib.types.SerializedResource;
import cwlib.types.data.Revision;
import cwlib.io.streams.MemoryInputStream;
import cwlib.resources.RGfxMaterial;
import java.io.*;
import java.nio.*;
import java.nio.file.*;
import java.util.*;
import java.security.MessageDigest;

public class MaterialOracle {
    static long unsigned(int n) { return Integer.toUnsignedLong(n); }
    static int count(MemoryInputStream s, int max) throws IOException {
        int n=s.i32();
        if(n<0 || n>max) throw new IOException("count budget");
        return n;
    }
    static Map<String,Object> descriptor(MemoryInputStream s) throws IOException {
        long flags=unsigned(s.i32()); int kind=s.i8()&255;
        if((kind&~3)!=0) throw new IOException("unknown descriptor kind");
        Long guid=(kind&2)!=0?unsigned(s.i32()):null;
        String hash=(kind&1)!=0?HexFormat.of().formatHex(s.bytes(20)):null;
        return TextureOracle.row("flags",flags,"kind",kind,"guid",guid,"sha1",hash);
    }
    static List<Integer> wraps(MemoryInputStream s) throws IOException {
        int n=count(s,8); List<Integer> values=new ArrayList<>();
        for(int i=0;i<n;i++) values.add(s.i8()&255);
        return values; // Preserve unknown numeric enums; do not silently choose defaults.
    }
    static Map<String,Object> prefix(MemoryInputStream s, Revision revision) throws IOException {
        int head=revision.getVersion();
        if(head<920 || head>951 || revision.getSubVersion()!=0 || revision.isVita() || revision.isToolkit())
            throw new IOException("prefix revision outside current PS3 sample contract");
        RGfxMaterial probe=new RGfxMaterial();
        probe.flags=s.i32();
        Map<String,Object> scalars=TextureOracle.row("flags",unsigned(probe.flags),
            "alpha_test_bits",unsigned(s.i32(true)),"alpha_layer_raw",s.i8()&255);
        probe.alphaMode=s.i8(); scalars.put("alpha_mode_raw",probe.alphaMode&255);
        scalars.put("shadow_cast_raw",s.i8()&255);
        for(String key:List.of("bump_level_bits","cosine_power_bits","reflection_blur_bits","refractive_index_bits"))
            scalars.put(key,unsigned(s.i32(true)));
        int slots=probe.getBlobOffsetCount(revision),shaders=probe.getShaderCount(revision);
        if(slots<1 || slots>24 || shaders!=slots) throw new IOException("unsupported shader table");
        List<Integer> offsets=new ArrayList<>();
        for(int i=0;i<slots;i++) offsets.add(s.i32());
        int bytes=count(s,8*1024*1024),start=s.getOffset(),previous=0;
        for(int offset:offsets) {
            if(offset<previous || offset>bytes) throw new IOException("shader interval out of blob");
            previous=offset;
        }
        s.seek(bytes,MemoryInputStream.SeekMode.Relative); // Never export/copy shader bytes.
        List<Object> textures=new ArrayList<>();
        for(int i=0;i<RGfxMaterial.MAX_TEXTURES;i++) textures.add(descriptor(s));
        List<Integer> wrapS=wraps(s),wrapT=wraps(s);
        // Toolkit fixed-width bytes() pads a short final read. Oracle guard rejects it.
        if(s.getOffset()>s.getBuffer().length) throw new IOException("truncated prefix");
        return TextureOracle.row("revision",head,"scalars",scalars,"shader_offset_count",slots,
            "shader_count",shaders,"shader_end_offsets",offsets,"blob_bytes",bytes,
            "blob_start",start,"uninterpreted_after_shader_bytes",bytes-previous,
            "textures",textures,"wrap_s_raw",wrapS,"wrap_t_raw",wrapT,
            "parsed_prefix_bytes",s.getOffset(),"unparsed_bytes",s.getBuffer().length-s.getOffset());
    }
    static void selfTest() throws Exception {
        // Authored synthetic fixed-width prefix: ten tiny opaque intervals, no graph.
        ByteBuffer b=ByteBuffer.allocate(27+40+4+10+40+24);
        b.putInt(0).putInt(0x3f000000).put((byte)0).put((byte)0).put((byte)1);
        for(int i=0;i<4;i++) b.putInt(0x3f800000);
        for(int i=1;i<=10;i++) b.putInt(i);
        b.putInt(10).put(new byte[10]);
        for(int i=0;i<8;i++) b.putInt(0).put((byte)0);
        for(int j=0;j<2;j++){b.putInt(8);for(int i=1;i<=8;i++)b.put((byte)i);}
        byte[] raw=Arrays.copyOf(b.array(),b.position());
        Map<String,Object> result=prefix(new MemoryInputStream(raw),new Revision(950));
        if(!result.get("blob_bytes").equals(10) || !result.get("unparsed_bytes").equals(0))
            throw new AssertionError("authored prefix");
        for(int length=0;length<raw.length;length++) {
            try {prefix(new MemoryInputStream(Arrays.copyOf(raw,length)),new Revision(950));}
            catch(Exception rejected){continue;}
            throw new AssertionError("accepted truncated prefix "+length);
        }
        ByteArrayOutputStream packed=new ByteArrayOutputStream();
        packed.write(0);packed.write(raw,4,23); // Integers vary; IEEE floats stay fixed width.
        for(int i=1;i<=10;i++) packed.write(i);
        packed.write(10);packed.write(new byte[10]);
        for(int i=0;i<8;i++){packed.write(0);packed.write(0);}
        for(int j=0;j<2;j++){packed.write(8);for(int i=1;i<=8;i++)packed.write(i);}
        byte[] compressedIntegers=packed.toByteArray();
        Map<String,Object> compressed=prefix(new MemoryInputStream(compressedIntegers,(byte)7),new Revision(950));
        for(String field:List.of("scalars","shader_end_offsets","textures","wrap_s_raw","wrap_t_raw"))
            if(!result.get(field).equals(compressed.get(field))) throw new AssertionError("compressed field "+field);
        for(int length=0;length<compressedIntegers.length;length++) {
            try {prefix(new MemoryInputStream(Arrays.copyOf(compressedIntegers,length),(byte)7),new Revision(950));}
            catch(Exception rejected){continue;}
            throw new AssertionError("accepted compressed truncated prefix "+length);
        }
        for(int kind=0;kind<=3;kind++) {
            ByteBuffer d=ByteBuffer.allocate(29).putInt(0).put((byte)kind);
            if((kind&2)!=0)d.putInt(17);
            if((kind&1)!=0)d.put(new byte[20]);
            Map<String,Object> parsed=descriptor(new MemoryInputStream(Arrays.copyOf(d.array(),d.position())));
            if(!parsed.get("kind").equals(kind) || ((parsed.get("guid")!=null)!=((kind&2)!=0)) ||
                ((parsed.get("sha1")!=null)!=((kind&1)!=0))) throw new AssertionError("descriptor presence");
        }
        byte[] bad=raw.clone();ByteBuffer.wrap(bad).putInt(27,11);
        try {prefix(new MemoryInputStream(bad),new Revision(950));}
        catch(IOException rejected){System.out.println("Synthetic material prefix bounds passed");return;}
        throw new AssertionError("invalid interval accepted");
    }
    public static void main(String[] args) throws Exception {
        selfTest();
        boolean diffuseMode=args.length==4&&args[3].equals("--diffuse");
        boolean tailMode=args.length==4&&args[3].equals("--tail");
        boolean graphMode=diffuseMode||tailMode||(args.length==4&&args[3].equals("--graph"));
        if(args.length!=3&&!graphMode) throw new IllegalArgumentException("FARC MANIFEST NEW-JSON [--graph|--tail]");
        if(graphMode)MaterialGraphOracle.selfTest();
        if(tailMode)MaterialTailOracle.selfTest();
        long deadline=System.nanoTime()+60_000_000_000L;
        Path output=Path.of(args[2]).toAbsolutePath();
        if(Files.exists(output) || !output.getParent().toRealPath().startsWith(Path.of("local-analysis").toRealPath()))
            throw new IOException("new ignored output required");
        JsonArray samples=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject().getAsJsonArray("samples");
        if(samples.size()>48) throw new IOException("sample budget");
        List<Object> results=new ArrayList<>();long total=0;
        try(RandomAccessFile f=new RandomAccessFile(args[0],"r")) {
            for(JsonElement item:samples) {
                MaterialTailOracle.check(()->System.nanoTime()>=deadline);
                JsonObject q=item.getAsJsonObject();String sha=q.get("sha1").getAsString();
                Map<String,Object> row=TextureOracle.row("sha1",sha);results.add(row);
                try {
                    long offset=q.get("offset").getAsLong(),size=q.get("size").getAsLong();
                    if(offset<0 || size<18 || size>1048576 || offset>f.length()-size || total+size>16*1024*1024)
                        throw new IOException("source budget");
                    byte[] raw=new byte[(int)size];f.seek(offset);f.readFully(raw);total+=size;
                    String actual=HexFormat.of().formatHex(MessageDigest.getInstance("SHA-1").digest(raw));
                    if(!actual.equals(sha)) throw new IOException("payload identifier mismatch");
                    ByteBuffer h=ByteBuffer.wrap(raw);
                    if(h.getInt()!=0x474d5462) throw new IOException("only clear GMTb");
                    int revision=h.getInt(),end=h.getInt(),branch=h.getInt(),flags=h.get()&255,compressed=h.get()&255;
                    if(revision<920 || revision>951 || branch!=0 || (flags!=0&&flags!=7) || compressed>1 || end<18 || end>raw.length-4)
                        throw new IOException("unsupported header/revision");
                    int dependencies=h.getInt(end);
                    if(dependencies<0 || dependencies>4096) throw new IOException("dependency budget");
                    byte[] clear=Arrays.copyOfRange(raw,18,end);
                    if(compressed!=0) {
                        if(clear.length<4 || TextureOracle.be16(clear,0)>1) throw new IOException("chunk flag");
                        byte[] wrapper=new byte[4+clear.length];System.arraycopy(new byte[]{'T','E','X',' '},0,wrapper,0,4);
                        System.arraycopy(clear,0,wrapper,4,clear.length);clear=TextureOracle.clearPayload(wrapper);
                    }
                    if(clear.length>8*1024*1024) throw new IOException("expanded budget");
                    SerializedResource resource=new SerializedResource(raw);
                    if(resource.getCompressionFlags()!=flags || !Arrays.equals(clear,resource.getStream().getBuffer()))
                        throw new IOException("wrapper disagreement");
                    MemoryInputStream stream=resource.getStream();
                    Map<String,Object> prefix=prefix(stream,resource.getRevision());
                    row.putAll(TextureOracle.row("status",graphMode?"parsed_graph":"parsed_prefix","compression_flags",flags,
                        "expanded_bytes",clear.length,"prefix",prefix));
                    if(graphMode)row.put("graph",MaterialGraphOracle.graph(stream,resource.getRevision()));
                    if(diffuseMode)row.put("diffuse_route",MaterialGraphOracle.lastDiffuseRoute);
                    if(diffuseMode)row.put("static_diffuse_multiply",MaterialGraphOracle.lastStaticMultiply);
                    if(tailMode) {
                        try {
                            row.put("tail",MaterialTailOracle.tail(stream,resource.getRevision(),()->System.nanoTime()>=deadline));
                            row.put("tail_status","parsed_tail");
                        } catch(Exception error) {
                            row.put("tail_status","rejected");
                            row.put("tail_error",error.getClass().getSimpleName()+": "+error.getMessage());
                        }
                    }
                } catch(Exception error) {row.put("status","rejected");row.put("error",error.getClass().getSimpleName()+": "+error.getMessage());}
            }
        }
        Files.writeString(output,new GsonBuilder().setPrettyPrinting().serializeNulls().create().toJson(
            TextureOracle.row("schema_version",1,"toolkit_commit","d6a2ef80b5a09999f08f23a803b516096055e5c9",
                "scope",tailMode?"PS3_GMT_tail_metadata_no_shader_bytes":graphMode?"PS3_GMT_graph_metadata_no_shader_bytes":"PS3_GMT_prefix_only_no_shader_bytes","sample_bytes",total,"samples",results)),StandardOpenOption.CREATE_NEW);
        System.out.println("Material prefix samples: "+results.size()+"; archived bytes: "+total);
    }
}
