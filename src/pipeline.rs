use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use std::thread::Scope;

pub trait PipelineIntermediateStage<T> {
    fn execute(&self, input_channel: Receiver<T>, output_channel: SyncSender<T>);
    fn get_input_buffer_size(&self) -> usize;
}

pub trait PipelineStageConsumer<T> {
    fn execute(&self, input_channel: Receiver<T>);
    fn get_input_buffer_size(&self) -> usize;
}

pub trait PipelineStageProducer<T> {
    fn execute(&self, output_channel: SyncSender<T>);
}

pub trait PipelineStep<T>: PipelineIntermediateStage<T> + Sync + Send {}
pub trait PipelineProducer<T>: PipelineStageProducer<T> + Sync + Send {}
pub trait PipelineConsumer<T>: PipelineStageConsumer<T> + Sync + Send {}

pub struct Pipeline<T> {
    stages: Vec<Box<dyn PipelineStep<T>>>,
    source: Option<Box<dyn PipelineProducer<T>>>,
    sink: Option<Box<dyn PipelineConsumer<T>>>
}

impl <T: Send>Pipeline<T> {
    pub fn new() -> Self{
        Pipeline{
            stages: Vec::new(),
            source: None,
            sink: None
        }
    }

    pub fn set_source(&mut self, source: impl PipelineProducer<T> + 'static) {
        self.source = Some(Box::new(source))
    }

    pub fn set_sink(&mut self, sink: impl PipelineConsumer<T> + 'static) {
        self.sink = Some(Box::new(sink));
    }

    pub fn push_stage(&mut self, stage: impl PipelineStep<T> + 'static){
        self.stages.push(Box::new(stage));
    }

    pub fn execute(&self){
        if self.source.is_none() {
            panic!("Source not set");
        }else if self.sink.is_none() {
            panic!("Sink not set");
        }
        thread::scope(|scope|{
            let (tx, rx) = mpsc::sync_channel(self.stages.first().map_or(self.sink.as_ref().unwrap().get_input_buffer_size(), |p|{p.get_input_buffer_size()}));
            scope.spawn(||{self.source.as_ref().unwrap().execute(tx)});
            self.construct_pipeline(scope, rx, 0);
        });
    }

    fn construct_pipeline<'a, 'b:'a>(&'a self, scope: &'b Scope<'a, '_>, rx: Receiver<T>, stage: usize) {
        match self.stages.get(stage) {
            Some(s) => {
                let old_rx = rx;
                let (tx, new_rx) = mpsc::sync_channel(self.stages.get(stage + 1).map_or(self.sink.as_ref().unwrap().get_input_buffer_size(), |s|{s.get_input_buffer_size()}));
                scope.spawn(||{s.execute(old_rx, tx)});
                self.construct_pipeline(scope, new_rx, stage + 1);
            },
            None => {
                scope.spawn(||{self.sink.as_ref().unwrap().execute(rx)});
            }
        }
    }
}