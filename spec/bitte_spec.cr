require "spec"
require "../src/cli.cr"

describe "arguments" do
  context "when not given" do
    it "should express gratitude" do
      File.tempfile("test") do |io|
        Bitte::CLI.run([] of String, error: io)
        io.rewind
        io.gets_to_end.should eq "Danke\n"
      end
    end
  end
end
