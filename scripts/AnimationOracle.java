// Authored bounded external-reader adapter. docs/animation-prefix.md records
// pinned RAnimation::serialize field ordering and limitations. No playback/export.
import com.google.gson.*;
import cwlib.types.SerializedResource;
import cwlib.types.data.Revision;
import cwlib.io.streams.MemoryInputStream;
import cwlib.io.serializer.Serializer;
import cwlib.resources.RAnimation;
import java.io.*;
import java.nio.*;
import java.nio.file.*;
import java.util.*;
import java.security.MessageDigest;

public class AnimationOracle {
    static void need(MemoryInputStream s,int n)throws IOException {
        if(n<0 || s.getOffset()<0 || s.getOffset()>s.getBuffer().length-n)throw new IOException("prefix bounds");
    }
    static List<Long> bits(MemoryInputStream s)throws IOException {
        need(s,16); List<Long> values=new ArrayList<>();
        for(int i=0;i<4;i++)values.add(Integer.toUnsignedLong(s.i32(true)));
        return values;
    }
    static List<Long> vectorBits(org.joml.Vector4f v) {
        return List.of(Integer.toUnsignedLong(Float.floatToRawIntBits(v.x)),Integer.toUnsignedLong(Float.floatToRawIntBits(v.y)),
            Integer.toUnsignedLong(Float.floatToRawIntBits(v.z)),Integer.toUnsignedLong(Float.floatToRawIntBits(v.w)));
    }
    static Map<String,Object> prefix(MemoryInputStream s,int revision)throws IOException {
        if(revision<888 || revision>1016)throw new IOException("unsupported animation prefix revision");
        need(s,14);
        int frames=s.u16(),fps=s.u16(),loop=s.u16(),morphs=s.u8();
        int[] counts=new int[6]; for(int i=0;i<5;i++)counts[i]=s.u8(); counts[5]=s.u16();
        List<Long> offset=null,scale=null; Integer fat=null;
        if(revision>907){offset=bits(s);scale=bits(s);if(revision>945){need(s,1);fat=s.u8();}}
        need(s,1); int length=s.i32();
        need(s,length); // Toolkit bytes(n) otherwise pads short reads.
        if(length>16*1024*1024)throw new IOException("animation byte array budget");
        return TextureOracle.row("revision",revision,"frames_raw_u16",frames,"fps_raw_u16",fps,
            "loop_start_raw_u16",loop,"morph_count_raw_u8",morphs,"counts",counts,
            "position_offset_bits",offset,"position_scale_bits",scale,"fat_raw_u8",fat,
            "animation_data_bytes",length,"animation_data_start",s.getOffset(),
            "parsed_prefix_bytes",s.getOffset(),"unparsed_bytes",s.getBuffer().length-s.getOffset());
    }
    static Map<String,Object> full(MemoryInputStream s,Revision revision,Map<String,Object> p)throws Exception {
        int frames=(int)p.get("frames_raw_u16"),morphs=(int)p.get("morph_count_raw_u8");
        int[] counts=(int[])p.get("counts"); int size=(int)p.get("animation_data_bytes");
        long rotations=counts[0]+(long)counts[1]*(frames-1),positions=counts[0]+(long)counts[2]*(frames-1),
            scales=counts[0]+(long)counts[3]*(frames-1),weights=morphs+(long)counts[4]*(frames-1);
        long base=8L*counts[0]+4L*counts[5]+counts[1]+counts[2]+counts[3]+counts[4];
        long required=((base+1)&~1L)+6L*(rotations+positions+scales)+2L*weights;
        if(frames<1 || frames>4096 || morphs>127 || rotations+positions+scales+weights>100000 || required>size)
            return TextureOracle.row("status","not_run_budget_or_layout");
        s.seek((int)p.get("animation_data_start")+size,MemoryInputStream.SeekMode.Begin);
        need(s,1);int locators=s.i32();need(s,0);
        if(locators!=0)return TextureOracle.row("status","not_run_nonempty_locator_array","locator_count_raw",locators);
        // Prefix/binary-byte lengths and all allocations are checked before invoking
        // unmodified RAnimation. Locator strings/arrays are outside this oracle scope.
        RAnimation animation=new Serializer(new MemoryInputStream(s.getBuffer(),s.getCompressionFlags()),revision).struct(null,RAnimation.class);
        boolean equal=Short.toUnsignedInt(animation.numFrames)==frames && Short.toUnsignedInt(animation.fps)==(int)p.get("fps_raw_u16")
            && Short.toUnsignedInt(animation.loopStart)==(int)p.get("loop_start_raw_u16") && Byte.toUnsignedInt(animation.morphCount)==morphs
            && animation.bones.length==counts[0] && animation.rotBonesAnimated.length==counts[1]
            && animation.posBonesAnimated.length==counts[2] && animation.scaledBonesAnimated.length==counts[3]
            && animation.morphsAnimated.length==counts[4];
        if(p.get("position_offset_bits")!=null)equal=equal && vectorBits(animation.posOffset).equals(p.get("position_offset_bits"))
            && vectorBits(animation.posScale).equals(p.get("position_scale_bits"));
        if(p.get("fat_raw_u8")!=null)equal=equal && Byte.toUnsignedInt(animation.fat)==(int)p.get("fat_raw_u8");
        if(!equal)throw new IOException("full reader scalar/count disagreement");
        return TextureOracle.row("status","reader_scalar_counts_equal","rotation_elements",animation.packedRotation.length,
            "position_elements",animation.packedPosition.length,"scale_elements",animation.packedScale.length,
            "morph_elements",animation.packedMorph.length,"locator_count",animation.locators.length);
    }
    static void selfTest()throws Exception {
        for(int revision:new int[]{888,907,908,945,946,951})for(byte flags:new byte[]{0,7}) {
            ByteBuffer b=ByteBuffer.allocate(64);b.putShort((short)3).putShort((short)24).putShort((short)1).put((byte)0);
            b.put(new byte[7]); if(revision>907){for(int i=0;i<8;i++)b.putInt(i<4?0:0x3f800000);if(revision>945)b.put((byte)0);}
            if(flags==0)b.putInt(0);else b.put((byte)0);int prefixLength=b.position();
            if(flags==0)b.putInt(0);else b.put((byte)0);
            byte[] raw=Arrays.copyOf(b.array(),b.position());
            MemoryInputStream s=new MemoryInputStream(raw,flags);Map<String,Object> p=prefix(s,revision);
            if(!full(s,new Revision(revision),p).get("status").equals("reader_scalar_counts_equal"))throw new AssertionError("reader guard");
            for(int n=0;n<prefixLength;n++) {
                try{prefix(new MemoryInputStream(Arrays.copyOf(raw,n),flags),revision);}catch(Exception expected){continue;}
                throw new AssertionError("accepted prefix truncation "+n);
            }
        }
        System.out.println("Animation prefix guards: revision gates, fixed/packed integer lengths, truncations and full reader counts passed");
    }
    public static void main(String[] args)throws Exception {
        if(args.length==1&&args[0].equals("--self-test")){selfTest();return;}
        if(args.length!=3)throw new IllegalArgumentException("FARC MANIFEST NEW-JSON");
        Path output=Path.of(args[2]).toAbsolutePath();
        if(Files.exists(output)||!output.getParent().toRealPath().startsWith(Path.of("local-analysis").toRealPath()))throw new IOException("new ignored output required");
        JsonObject manifest=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        if(!manifest.get("toolkit_commit").getAsString().equals("d6a2ef80b5a09999f08f23a803b516096055e5c9"))throw new IOException("pin differs");
        JsonArray samples=manifest.getAsJsonArray("samples");if(samples.size()>48)throw new IOException("sample budget");
        List<Object> results=new ArrayList<>();long total=0,deadline=System.nanoTime()+60_000_000_000L;
        try(RandomAccessFile f=new RandomAccessFile(args[0],"r")) {
            for(JsonElement item:samples) {
                if(System.nanoTime()>deadline)throw new IOException("oracle time budget");
                JsonObject q=item.getAsJsonObject();String sha=q.get("sha1").getAsString();Map<String,Object> row=TextureOracle.row("sha1",sha);results.add(row);
                try {
                    if(!TextureOracle.sameArchive(q.get("source_path").getAsString(),args[0]))throw new IOException("source differs");
                    long at=q.get("offset").getAsLong(),size=q.get("size").getAsLong();
                    if(at<0||size<18||size>2*1024*1024||at>f.length()-size||total+size>64*1024*1024)throw new IOException("read budget");
                    byte[] raw=new byte[(int)size];f.seek(at);f.readFully(raw);total+=size;
                    if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-1").digest(raw)).equals(sha))throw new IOException("identifier mismatch");
                    ByteBuffer h=ByteBuffer.wrap(raw);
                    if(h.getInt()!=0x414e4d62)throw new IOException("only clear ANMb");
                    int revision=h.getInt(),end=h.getInt(),branch=h.getInt(),flags=h.get()&255,compressed=h.get()&255;
                    if(revision<888||revision>1016||branch!=0||(flags!=0&&flags!=7)||compressed>1||end<18||end>raw.length-4)
                        throw new IOException("unsupported header/revision");
                    // This corpus has no ANM dependency entries. Do not hand an
                    // unbounded dependency count to the community constructor.
                    if(h.getInt(end)!=0)throw new IOException("nonempty dependency table outside oracle scope");
                    byte[] clear=Arrays.copyOfRange(raw,18,end);
                    if(compressed!=0){byte[] wrapped=new byte[clear.length+4];System.arraycopy(new byte[]{'T','E','X',' '},0,wrapped,0,4);System.arraycopy(clear,0,wrapped,4,clear.length);clear=TextureOracle.clearPayload(wrapped);}
                    if(clear.length>16*1024*1024)throw new IOException("expanded budget");
                    SerializedResource resource=new SerializedResource(raw);
                    if(resource.getCompressionFlags()!=flags||!Arrays.equals(clear,resource.getStream().getBuffer()))throw new IOException("wrapper disagreement");
                    MemoryInputStream s=resource.getStream();Map<String,Object> p=prefix(s,revision);
                    row.putAll(TextureOracle.row("status","parsed_prefix","expanded_bytes",clear.length,"compression_flags",flags,"prefix",p));
                    try{row.put("full_reader",full(s,resource.getRevision(),p));}catch(Exception e){row.put("full_reader",TextureOracle.row("status","reader_failed","reason",e.getClass().getSimpleName()+": "+e.getMessage()));}
                }catch(Exception e){row.putAll(TextureOracle.row("status","rejected","reason",e.getClass().getSimpleName()+": "+e.getMessage()));}
            }
        }
        Files.writeString(output,new GsonBuilder().setPrettyPrinting().serializeNulls().create().toJson(TextureOracle.row(
            "schema_version",1,"toolkit_commit","d6a2ef80b5a09999f08f23a803b516096055e5c9","scope","animation_prefix_metadata_no_playback",
            "sample_bytes",total,"samples",results)),StandardOpenOption.CREATE_NEW);
        System.out.println("Animation oracle: "+results.size()+" samples, "+total+" archived bytes");
    }
}
